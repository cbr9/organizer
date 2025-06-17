use itertools::Itertools;
use std::{
	collections::HashMap,
	path::PathBuf,
	process::Command,
	sync::{Arc, RwLock},
};

use crate::config::{actions::common::enabled, context::ExecutionContext};
use anyhow::{bail, Result};
use lettre::{
	message::{header::ContentType, Attachment, Mailbox, MessageBuilder, MultiPart, SinglePart},
	transport::smtp::authentication::Credentials,
	SmtpTransport,
	Transport,
};
use serde::{Deserialize, Serialize};

use crate::{resource::Resource, templates::template::Template};

use super::Action;

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

impl Email {
	fn get_or_insert_credentials(&self, cache: &Arc<RwLock<HashMap<Mailbox, Credentials>>>) -> anyhow::Result<Credentials> {
		tracing::info!("Getting credentials for external tool.");
		if let Ok(reader) = cache.read() {
			if let Some(creds) = reader.get(&self.sender) {
				return Ok(creds.clone());
			}
		}

		if let Ok(mut writer) = cache.write() {
			// Another thread might have acquired the
			// write lock and inserted the key while we were waiting.
			if let Some(creds) = writer.get(&self.sender) {
				return Ok(creds.clone());
			}
			let command_and_args = self.password_cmd.split(" ").collect_vec();
			let executable = command_and_args[0];

			let args = &command_and_args[1..];
			let mut command = Command::new(executable);
			let output = command.args(args).output()?;
			let password = String::from_utf8(output.stdout)?.trim().to_string();
			let creds = Credentials::new(self.sender.email.to_string(), password);

			// Insert the new credentials into the cache.
			writer.insert(self.sender.clone(), creds.clone());

			return Ok(creds);
		}
		bail!("Could not acquire email cache")
	}
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

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>> {
		if !ctx.settings.dry_run && self.enabled {
			let mut email = MessageBuilder::new()
				.from(self.sender.clone())
				.to(self.recipient.clone())
				.date_now();

			let context = ctx.services.template_engine.new_context(res);
			if let Some(subject) = &self.subject {
				if let Some(subject) = ctx.services.template_engine.render(subject, &context)? {
					email = email.subject(subject);
				}
			}

			let mut multipart = MultiPart::mixed().build();

			// Add body if it exists
			if let Some(body) = &self.body {
				if let Some(body) = ctx.services.template_engine.render(body, &context)? {
					multipart = multipart.singlepart(SinglePart::plain(body));
				}
			}

			if self.attach {
				if let Some(mime) = mime_guess::from_path(res.path()).first() {
					let content = std::fs::read(res.path())?;
					let content_type = ContentType::from(mime);
					let attachment = Attachment::new(res.path().file_name().unwrap().to_string_lossy().to_string()).body(content, content_type);

					multipart = multipart.singlepart(attachment);
				}
			};

			let email = email.multipart(multipart)?;

			let creds = self.get_or_insert_credentials(&ctx.services.credential_cache)?;
			let mailer = SmtpTransport::relay(&self.smtp_server).unwrap().credentials(creds).build();

			if let Err(e) = mailer.send(&email) {
				tracing::error!("Could not send email: {:?}", e);
				return Ok(None);
			};
		}
		Ok(Some(res.path().to_path_buf()))
	}
}
