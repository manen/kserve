use std::{borrow::Cow, path::Path};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::config;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	pub addr: Cow<'static, str>,
	pub port: u16,

	pub serve_directory: Cow<'static, str>,
	pub allow_indexing: bool,
}
impl Default for Config {
	fn default() -> Self {
		Config {
			addr: "0.0.0.0".into(),
			port: 9090,

			serve_directory: ".".into(),
			allow_indexing: true,
		}
	}
}

impl Config {
	/// reads existing or creates a config at path
	pub async fn new(config_path: impl AsRef<Path>) -> anyhow::Result<Self> {
		let config_path = config_path.as_ref();

		let read = Self::read(config_path).await;
		match read {
			None => Self::create(config_path).await,
			Some(read) => read,
		}
	}

	/// returns none if config doesn't exist, returns Some(Err(_)) if there was an other error
	pub async fn read(config_path: impl AsRef<Path>) -> Option<anyhow::Result<Self>> {
		let config_path = config_path.as_ref();

		let file = tokio::fs::read_to_string(&config_path).await;
		let file = match file {
			Ok(a) => a,
			std::io::Result::Err(err) => match err.kind() {
				std::io::ErrorKind::NotFound => return None,
				_ => {
					return Some(
						Err(err)
							.with_context(|| format!("while reading {}", config_path.display())),
					);
				}
			},
		};

		let parsed: anyhow::Result<Config> = toml::from_str(&file)
			.with_context(|| format!("while deserializing {}", config_path.display()));
		Some(parsed)
	}
	/// creates a new config and returns the config it wrote
	pub async fn create(config_path: impl AsRef<Path>) -> anyhow::Result<Self> {
		let config_path = config_path.as_ref();

		let config = Config::default();
		let file =
			toml::to_string(&config).with_context(|| format!("while serializing {config:?}"))?;

		tokio::fs::write(&config_path, &file)
			.await
			.with_context(|| format!("while writing config to {}", config_path.display()))?;

		Ok(config)
	}
}
