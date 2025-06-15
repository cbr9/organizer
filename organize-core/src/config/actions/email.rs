use itertools::Itertools;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use crate::config::actions::common::enabled;
use anyhow::Result;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MessageBuilder, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::resource::Resource;
use crate::templates::template::Template;
use crate::templates::TemplateEngine;

use super::Action;

static CREDENTIALS: LazyLock<Mutex<HashMap<Mailbox, Credentials>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Deserialize, Serialize, Eq, PartialEq, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Email {
	sender: Mailbox,
	password_cmd: String,
	recipient: Mailbox,
	smtp_server: String,
	subject: Option<Template>,
	body: Option<Template>,
	#[serde(default)]
	attach: bool,
	#[serde(default = "enabled")]
	pub enabled: bool,
}

#[typetag::serde(name = "email")]
impl Action for Email {
	fn templates(&self) -> Vec<&Template> {
		let mut templates = vec![];
		if let Some(subject) = &self.subject {
			templates.push(subject);
		}
		if let Some(body) = &self.body {
			templates.push(body);
		}
		templates
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(template_engine))]
	fn execute(&self, res: &Resource, template_engine: &TemplateEngine, dry_run: bool) -> Result<Option<PathBuf>> {
		if !dry_run && self.enabled {
			let mut email = MessageBuilder::new()
				.from(self.sender.clone())
				.to(self.recipient.clone())
				.date_now();

			let context = template_engine.new_context(res);
			if let Some(subject) = &self.subject {
				let subject = template_engine.render(subject, &context)?;
				email = email.subject(subject);
			}

			let mut multipart = MultiPart::mixed().build();

			// Add body if it exists
			if let Some(body) = &self.body {
				let body = template_engine.render(body, &context)?;
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
		}
		Ok(Some(res.path.to_path_buf()))
	}
}
