use colored::Colorize;
use itertools::Itertools;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::sync::Mutex;

use anyhow::Result;
use lazy_static::lazy_static;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MessageBuilder, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::Address;
use lettre::{Message, SmtpTransport, Transport};
use serde::Deserialize;

use crate::resource::Resource;
use crate::templates::Template;

use super::script::ActionConfig;
use super::AsAction;

lazy_static! {
	static ref CREDENTIALS: Mutex<HashMap<Mailbox, Credentials>> = Mutex::new(HashMap::new());
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct Email {
	sender: Mailbox,
	password_cmd: String,
	recipient: Mailbox,
	smtp_server: String,
	subject: Option<Template>,
	body: Option<Template>,
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

		let mut multipart = MultiPart::mixed().build();

		// Add body if it exists
		if let Some(body) = &self.body {
			let body = body.render(&res.context).unwrap();
			multipart = multipart.singlepart(SinglePart::plain(body));
		}

		if self.attach {
			if let Some(mime) = mime_guess::from_path(&res.path).first() {
				let content = std::fs::read(&res.path)?;
				let content_type = ContentType::from(mime);
				let attachment = Attachment::new(res.path.file_name().unwrap().to_string_lossy().to_string()).body(content, content_type);

				multipart = multipart.singlepart(attachment);
			}
		};

		let email = email.multipart(multipart)?;

		let creds = {
			let mut lock = CREDENTIALS.lock().unwrap();
			if let Some(creds) = lock.get(&self.sender) {
				creds.clone()
			} else {
				let command_and_args = self.password_cmd.split(" ").collect_vec();
				let executable = command_and_args[0];

				let args = &command_and_args[1..];
				let mut command = Command::new(executable);
				let output = command.args(args).output()?;
				let password = String::from_utf8(output.stdout)?.trim().to_string();
				let creds = Credentials::new(self.sender.email.to_string(), password);
				lock.insert(self.sender.clone(), creds.clone());
				creds
			}
		};

		let mailer = SmtpTransport::relay(&self.smtp_server).unwrap().credentials(creds).build();

		match mailer.send(&email) {
			Ok(_) => tracing::info!("Email sent successfully!"),
			Err(e) => tracing::error!("Could not send email: {:?}", e),
		};

		Ok(Some(res.path.to_path_buf()))
	}
}
