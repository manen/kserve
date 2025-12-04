use std::{
	borrow::Cow,
	io::ErrorKind,
	path::{Component, Path, PathBuf},
};

use anyhow::{Context, anyhow};

use crate::{Config, Frame};

fn normalize_without_escape(user: &Path) -> Option<PathBuf> {
	let mut stack = Vec::new();

	for comp in user.components() {
		match comp {
			Component::ParentDir => {
				if stack.is_empty() {
					// would escape
					return None;
				}
				stack.pop();
			}
			Component::Normal(s) => stack.push(s.to_owned()),
			Component::CurDir => {}
			Component::RootDir | Component::Prefix(_) => return None,
		}
	}

	Some(stack.iter().collect())
}

fn join_without_escape(a: &Path, untrusted_b: &Path) -> anyhow::Result<PathBuf> {
	let b = normalize_without_escape(untrusted_b);
	let b = b
		.with_context(|| {
			format!(
				"failed to normalize {} without it leaking into the outer fs",
				untrusted_b.display()
			)
		})
		.with_context(|| format!("while appending it to {}", a.display()))?;

	Ok(a.join(b))
}

#[derive(Clone, Debug)]
pub struct Dir {
	path: PathBuf,
}
impl Dir {
	pub fn new(path: PathBuf) -> Self {
		Self { path }
	}

	pub async fn get_config_only(&self) -> Option<anyhow::Result<Config>> {
		let config_path = self.path.join("_kserve.toml");

		let file = tokio::fs::read_to_string(&config_path).await;
		let file = match file {
			Ok(a) => a,
			Err(err) => match err.kind() {
				ErrorKind::NotFound => return None,
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
	pub async fn create_config(&self) -> anyhow::Result<()> {
		let config_path = self.path.join("_kserve.toml");

		let config = Config::default();
		let file =
			toml::to_string(&config).with_context(|| format!("while serializing {config:?}"))?;

		tokio::fs::write(&config_path, &file)
			.await
			.with_context(|| format!("while writing config to {}", config_path.display()))?;

		Ok(())
	}

	/// read or create config
	pub async fn config(&self) -> anyhow::Result<Config> {
		let config = self.get_config_only().await;
		match config {
			Some(a) => return a,
			None => {
				self.create_config().await?;
				Ok(Config::default())
			}
		}
	}

	/// non-absolute, dir-specific path
	pub async fn handle_path(
		&self,
		path: &Path,
		config: &Config,
	) -> anyhow::Result<(Cow<'static, str>, Vec<u8>)> {
		let joined = join_without_escape(&self.path, path)?;
		println!("{}", joined.display());

		let metadata = tokio::fs::metadata(&joined)
			.await
			.with_context(|| format!("while querying metadata for {}", path.display()))?;

		if metadata.is_dir() {
			if !config.allow_indexing {
				return Ok((
					"text/plain".into(),
					"this server does not allow indexing".into(),
				));
			}

			let nav = async {
				let top = path
					.iter()
					.rev()
					.next()
					.map(|a| a.to_string_lossy())
					.map(|a| format!("{a}/"))
					.unwrap_or_else(String::new);

				let mut readdir = tokio::fs::read_dir(&joined)
					.await
					.with_context(|| format!("while reading dir {}", joined.display()))?;

				let mut buf = Vec::new();

				loop {
					let entry = readdir.next_entry().await.with_context(|| {
						format!("while reading next entry from readdir {}", joined.display())
					})?;
					match entry {
						Some(entry) => {
							let name = entry.file_name();
							let name = name.to_string_lossy();
							let name = match name {
								Cow::Borrowed(a) => a.to_string(),
								Cow::Owned(a) => a,
							};

							buf.push(name);
						}
						None => break,
					}
				}

				let entries = buf
					.into_iter()
					.map(|filename| {
						format!("<div><a href=\"{top}{filename}\">{filename}</a></div>")
					})
					.collect::<String>();
				let nav = format!(
					"<div><div>{}</div><nav>{entries}</nav></div>",
					path.display()
				);

				anyhow::Ok(nav)
			};
			let frame = self.resolve_frame(&joined);

			let readme = async {
				let readme_path = joined.join("README.md");
				let html = self.handle_md_raw(&readme_path).await;

				match html {
					Ok(a) => Some(a),
					Err(err) => {
						if false {
							eprintln!(
								"failed to prerender readme for {}:\n{}",
								joined.display(),
								err
							);
						}
						None
					}
				}
			};

			let (nav, frame, readme) = tokio::join!(nav, frame, readme);
			let (nav, frame, readme) = (
				nav.with_context(|| format!("while creating navbar for {}", joined.display()))?,
				frame.with_context(|| format!("while resolving frame for {}", joined.display()))?,
				readme,
			);

			let readme = readme
				.map(|rm| format!("<main>{rm}</main>"))
				.unwrap_or_default();

			let content = format!("<div>{nav} <br> {readme}</div>");

			return Ok(("text/html".into(), frame.with_content(&content).into()));
		}

		if metadata.is_file() {
			return self
				.handle_file(&joined)
				.await
				.with_context(|| format!("while handling {}", path.display()));
		}

		Err(anyhow!(
			"this path exists, but it's neither a directory or a file"
		))
	}

	/// returns (mime, content) \
	/// absolute path
	pub async fn handle_file(&self, path: &Path) -> anyhow::Result<(Cow<'static, str>, Vec<u8>)> {
		// return Ok(("text/plain".into(), format!("{}", path.display()).into()));

		let handle_as_md = path
			.extension()
			.map(|a| a.to_string_lossy() == "md")
			.unwrap_or(false);

		println!("{}: {handle_as_md}", path.display()); // ts dont work
		if handle_as_md {
			let html = self
				.handle_md(path)
				.await
				.with_context(|| format!("while handling {} as markdown", path.display()))?;

			Ok(("text/html".into(), html.into()))
		} else {
			let (mime, content) = self
				.handle_non_md(path)
				.await
				.with_context(|| format!("while handling {} as non md", path.display()))?;

			Ok((mime, content))
		}
	}

	/// expects absolute, pre-joined path
	async fn resolve_frame(&self, file_path: &Path) -> anyhow::Result<Frame> {
		let mut path = file_path.to_path_buf();

		let is_dir = tokio::fs::metadata(&path)
			.await
			.map(|md| md.is_dir())
			.ok()
			.unwrap_or(false);
		if is_dir {
			path.push("you_should_not_see_this_ever_if_you_do_its_a_bug.txt")
		}

		let mut frame = Frame::default();

		while path.pop() {
			let frame_path = path.join("_frame.html");

			let file = tokio::fs::read_to_string(&frame_path).await;
			let file = match file {
				Ok(a) => a,
				Err(err) => match err.kind() {
					ErrorKind::NotFound => continue,
					_ => {
						return Err(err)
							.with_context(|| format!("while reading {}", frame_path.display()));
					}
				},
			};

			let new_frame = Frame::new(file)
				.with_context(|| format!("while turning {} into a frame", frame_path.display()))?;
			frame = frame.with_child(&new_frame);
		}

		Ok(frame)
	}

	/// expects absolute, pre-joined path
	async fn handle_md_raw(&self, path: &Path) -> anyhow::Result<String> {
		let md = tokio::fs::read_to_string(&path)
			.await
			.with_context(|| format!("while reading {} as string", path.display()))?;

		let opts = comrak::Options::default();
		let html = comrak::markdown_to_html(&md, &opts);

		Ok(html)
	}

	/// absolute path
	async fn handle_md(&self, path: &Path) -> anyhow::Result<String> {
		let html_body = self.handle_md_raw(&path);
		let frame = self.resolve_frame(&path);

		let (html_body, frame) = tokio::join!(html_body, frame);
		let (html_body, frame) = (
			html_body.with_context(|| "while resolving html body")?,
			frame.with_context(|| "while reading _frame.html")?,
		);

		Ok(frame.with_content(&html_body))
	}
	/// absolute path
	async fn handle_non_md(&self, path: &Path) -> anyhow::Result<(Cow<'static, str>, Vec<u8>)> {
		let mime = mime_guess::from_path(path).first_or_octet_stream();
		let mime = mime.essence_str();

		let content = tokio::fs::read(&path)
			.await
			.with_context(|| format!("while reading {} as binary", path.display()))?;

		Ok((mime.to_string().into(), content))
	}
}
