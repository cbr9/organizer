use anyhow::{Result, anyhow};
use async_trait::async_trait;
use clap::Parser;
use organize_lib::{
	config::{
		actions::{Input, UndoConflict, UndoError, UndoSettings},
		context::RunSettings,
	},
	journal::Journal,
};

use super::Cmd;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Undo {
	/// The ID of the session to undo.
	#[arg(long, conflicts_with = "last_session")]
	session_id: Option<i64>,

	/// Use the most recent session.
	#[arg(long, default_value_t = true)]
	last_session: bool,

	// If there is a name collision conflict while undoing, ask me what to do
	#[arg(long, short = 'i', conflicts_with_all = &["on_conflict"])]
	interactive: bool,

	#[arg(long, value_enum, default_value_t = UndoConflict::Abort)]
	on_conflict: UndoConflict,
}

#[async_trait]
impl Cmd for Undo {
	async fn run(self) -> Result<()> {
		let settings = RunSettings { dry_run: false };
		let journal = Journal::new(&settings).await?; // Assumes a simple ::new()

		let settings = UndoSettings {
			interactive: self.interactive,
			on_conflict: self.on_conflict,
		};
		let target_id = if self.last_session {
			journal
				.get_last_session_id()
				.await?
				.ok_or_else(|| anyhow!("No sessions found in the journal."))?
		} else {
			self.session_id.unwrap()
		};

		let transactions = journal.get_pending_transactions_for_session(target_id).await?;

		if transactions.is_empty() {
			println!("No pending operations to undo for session {target_id}.");
			return Ok(());
		}

		for transaction in &transactions {
			for undo_op in &transaction.receipt.undo {
				if undo_op.verify().await.is_ok() {
					match undo_op.undo(&settings).await {
						Ok(_) => {
							journal.update_transaction_undo_status(transaction.id, "DONE").await?;
							tracing::info!("Transaction {} undone.", transaction.id);
						}
						Err(e) => {
							if matches!(e, UndoError::Abort) {
								let inputs = transaction
									.receipt
									.inputs
									.iter()
									.map(|input: &Input| match input {
										Input::Processed(resource) => resource.to_string_lossy().to_string(),
										Input::Skipped(resource) => resource.to_string_lossy().to_string(),
									})
									.collect::<Vec<String>>()
									.join("\n -");

								eprintln!(
									"There was a conflict undoing transaction {}.\nOne of the following files may already exist: \n - {}\nAborting \
									 undo process. Run in interactive mode or choose a default conflict resolution strategy. You can also move the \
									 file manually.",
									transaction.id, inputs
								);
								return Ok(());
							}

							eprintln!("Failed to undo transaction {}: {}", transaction.id, e);
							return Err(e.into());
						}
					}
				}
			}
		}

		Ok(())
	}
}
