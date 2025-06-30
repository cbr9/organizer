use crate::{
	action::{Action, Receipt},
	context::RunSettings,
};
use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::time::{SystemTime, UNIX_EPOCH};

/// The Journal service, responsible for all database interactions.
#[derive(Debug, Clone)]
pub struct Journal {
	pool: SqlitePool,
}

#[derive(Debug)]
pub struct Transaction {
	pub id: i64,
	pub receipt: Receipt,
}

impl Journal {
	/// Creates a new Journal instance, connects to the database, and runs migrations.
	pub async fn new(settings: &RunSettings) -> Result<Self> {
		let db_url = if settings.dry_run {
			// For a dry run, use a temporary, private in-memory SQLite database.
			"sqlite::memory:".to_string()
		} else {
			// For a real run, use the persistent database file specified in .env.
			dotenvy::dotenv().ok();
			std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for real runs")
		};

		let pool = SqlitePoolOptions::new().max_connections(5).connect(&db_url).await?;
		sqlx::migrate!("./migrations").run(&pool).await?;

		Ok(Self { pool })
	}

	pub async fn start_session(&self) -> Result<i64> {
		let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

		let record = sqlx::query!(
			r#"
            INSERT INTO sessions (start_time, status)
            VALUES (?1, 'running')
            "#,
			now,
		)
		.execute(&self.pool)
		.await?;

		Ok(record.last_insert_rowid())
	}

	#[allow(clippy::borrowed_box)]
	pub async fn record_transaction(&self, session_id: i64, action: &Box<dyn Action>, receipt: &Receipt) -> Result<()> {
		if receipt.undo.is_empty() {
			return Ok(());
		}

		let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
		let action_type = action.typetag_name();

		let action_data = serde_json::to_string(action)?;
		let receipt_data = serde_json::to_string(receipt)?;

		sqlx::query!(
			r#"
            INSERT INTO transactions (session_id, type, action, receipt, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
			session_id,
			action_type,
			action_data,
			receipt_data,
			now
		)
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	pub async fn get_pending_transactions_for_session(&self, session_id: i64) -> Result<Vec<Transaction>> {
		let transactions = sqlx::query_as!(
			Transaction,
			"SELECT id, receipt FROM transactions WHERE session_id = ? AND undo_status = 'PENDING' ORDER BY timestamp DESC",
			session_id
		)
		.fetch_all(&self.pool)
		.await?;

		Ok(transactions)
	}

	pub async fn update_transaction_undo_status(&self, transaction_id: i64, status: &str) -> Result<()> {
		sqlx::query!("UPDATE transactions SET undo_status = ? WHERE id = ?", status, transaction_id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}

	/// Marks a session as completed with a final status.
	pub async fn end_session(&self, session_id: i64, status: &str) -> Result<()> {
		let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
		sqlx::query!("UPDATE sessions SET end_time = ?1, status = ?2 WHERE id = ?3", now, status, session_id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}

	pub async fn get_last_session_id(&self) -> Result<Option<i64>> {
		let result = sqlx::query!("SELECT id FROM sessions ORDER BY start_time DESC LIMIT 1")
			.fetch_optional(&self.pool)
			.await?;
		Ok(result.map(|row| row.id))
	}
}
