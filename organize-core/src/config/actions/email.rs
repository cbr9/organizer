use std::{
	collections::HashMap,
	path::PathBuf,
	process::Command,
	sync::{Arc, RwLock},
};

use crate::{
	config::{actions::common::enabled, context::ExecutionContext},
	errors::{ActionError, ErrorContext},
};
use anyhow::Result;
use lettre::{
	message::{header::ContentType, Attachment, Mailbox, MessageBuilder, MultiPart, SinglePart},
	transport::smtp::authentication::Credentials,
	SmtpTransport,
	Transport,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

// An error type specific to the Email action
#[derive(Error, Debug)]
pub enum EmailError {
	#[error("SMTP connection failed")]
	SmtpFailure(#[from] lettre::transport::smtp::Error),

	#[error("Could not get credentials from `password_cmd`")]
	Credentials(#[from] std::io::Error),

	#[error("Invalid password_cmd: '{0}'")]
	InvalidPasswordCommand(String),

	#[error("Invalid password: password cannot be empty")]
	InvalidPassword(#[from] std::string::FromUtf8Error),

	#[error(transparent)]
	EmailError(#[from] lettre::error::Error),

	#[error("Could not acquire cached credentials")]
	Cache,
}

impl Email {
	#[tracing::instrument(err)]
	fn get_or_insert_credentials(&self, cache: &Arc<RwLock<HashMap<Mailbox, Credentials>>>) -> Result<Credentials, EmailError> {
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
			let parts = shlex::split(&self.password_cmd).ok_or_else(|| EmailError::InvalidPasswordCommand(self.password_cmd.clone()))?;

			if parts.is_empty() {
				return Err(EmailError::InvalidPasswordCommand("password command cannot be empty".to_string()));
			}

			let executable = &parts[0];
			let args = &parts[1..];

			let output = Command::new(executable).args(args).output().map_err(EmailError::Credentials)?;
			let password = String::from_utf8(output.stdout).map_err(EmailError::InvalidPassword)?;
			let creds = Credentials::new(self.sender.email.to_string(), password);

			// Insert the new credentials into the cache.
			writer.insert(self.sender.clone(), creds.clone());

			return Ok(creds);
		}

		Err(EmailError::Cache)
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
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>, ActionError> {
		if !ctx.settings.dry_run && self.enabled {
			let mut email = MessageBuilder::new()
				.from(self.sender.clone())
				.to(self.recipient.clone())
				.date_now();

			let context = ctx
				.services
				.templater
				.context()
				.path(res.path())
				.root(res.root())
				.build(&ctx.services.templater);

			if let Some(subject) = &self.subject {
				let maybe_rendered = ctx
					.services
					.templater
					.render(subject, &context)
					.map_err(|e| ActionError::Template {
						source: e,
						template: subject.clone(),
						context: ErrorContext::from_scope(&ctx.scope),
					})?;

				if let Some(rendered) = maybe_rendered {
					email = email.subject(rendered);
				}
			}

			let mut multipart = MultiPart::mixed().build();

			// Add body if it exists
			if let Some(body) = &self.body {
				let maybe_rendered = ctx
					.services
					.templater
					.render(body, &context)
					.map_err(|e| ActionError::Template {
						source: e,
						template: body.clone(),
						context: ErrorContext::from_scope(&ctx.scope),
					})?;

				if let Some(rendered) = maybe_rendered {
					multipart = multipart.singlepart(SinglePart::plain(rendered));
				}
			}

			if self.attach {
				if let Some(mime) = mime_guess::from_path(res.path()).first() {
					let content = std::fs::read(res.path()).map_err(|e| ActionError::Io {
						source: e,
						path: res.path().to_path_buf(),
						target: None,
						context: ErrorContext::from_scope(&ctx.scope),
					})?;
					let content_type = ContentType::from(mime);
					let attachment = Attachment::new(res.path().file_name().unwrap().to_string_lossy().to_string()).body(content, content_type);

					multipart = multipart.singlepart(attachment);
				}
			};

			let email = email.multipart(multipart).map_err(|e| ActionError::Email {
				source: EmailError::EmailError(e),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;

			let creds = self
				.get_or_insert_credentials(&ctx.services.blackboard.credentials)
				.map_err(|e| ActionError::Email {
					source: e,
					context: ErrorContext::from_scope(&ctx.scope),
				})?;

			let mailer = SmtpTransport::relay(&self.smtp_server)
				.map_err(|e| ActionError::Email {
					source: EmailError::SmtpFailure(e),
					context: ErrorContext::from_scope(&ctx.scope),
				})?
				.credentials(creds)
				.build();

			let _response = mailer.send(&email).map_err(|e| ActionError::Email {
				source: EmailError::SmtpFailure(e),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;
		}

		Ok(Some(res.path().to_path_buf()))
	}
}
