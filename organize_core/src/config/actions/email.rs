use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MessageBuilder};
use lettre::transport::smtp::authentication::Credentials;
use lettre::Address;
use lettre::{Message, SmtpTransport, Transport};
use serde::Deserialize;

use crate::resource::Resource;
use crate::templates::Template;

use super::script::ActionConfig;
use super::AsAction;

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct Email {
	sender: Mailbox,
	password: String,
	recipient: Mailbox,
	subject: Option<Template>,
	#[serde(default)]
	attach: bool,
}

impl AsAction for Email {
	const CONFIG: ActionConfig = ActionConfig {
		requires_dest: false,
		parallelize: true,
	};

	#[tracing::instrument(err(Debug), skip(_dest))]
	fn execute<T: AsRef<std::path::Path>>(&self, res: &Resource, _dest: Option<T>, dry_run: bool) -> Result<Option<PathBuf>> {
		let mut email = MessageBuilder::new()
			.from(self.sender.clone())
			.to(self.recipient.clone())
			.date_now();

		if let Some(subject) = &self.subject {
			email = email.subject(subject.render(&res.context)?);
		}

		let attachment = if self.attach {
			if let Some(mime) = mime_guess::from_path(&res.path).first() {
				let content = std::fs::read(&res.path)?;
				let content_type = ContentType::from(mime);
				Some(Attachment::new(res.path.file_name().unwrap().to_string_lossy().to_string()).body(content, content_type))
			} else {
				None
			}
		} else {
			None
		};

		let email = if let Some(attachment) = attachment {
			email.singlepart(attachment)?
		} else {
			email.body(vec![])?
		};

		let creds = Credentials::new(self.sender.email.to_string(), self.password.clone());

		let mailer = SmtpTransport::relay("smtp.gmail.com").unwrap().credentials(creds).build();

		match mailer.send(&email) {
			Ok(_) => tracing::info!("Email sent successfully!"),
			Err(e) => tracing::error!("Could not send email: {:?}", e),
		};

		Ok(Some(res.path.to_path_buf()))
	}
}
