use std::{
	borrow::Cow,
	io::ErrorKind,
	path::{Component, Path, PathBuf},
};

use anyhow::Context;

use crate::Frame;

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

	/// returns (mime, content)
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

	async fn handle_md(&self, path: &Path) -> anyhow::Result<String> {
		let joined = join_without_escape(&self.path, path)?;

		let html_body = self.handle_md_raw(&joined);
		let frame = self.resolve_frame(&joined);

		let (html_body, frame) = tokio::join!(html_body, frame);
		let (html_body, frame) = (
			html_body.with_context(|| "while resolving html body")?,
			frame.with_context(|| "while reading _frame.html")?,
		);

		Ok(frame.with_content(&html_body))
	}
	async fn handle_non_md(&self, path: &Path) -> anyhow::Result<(Cow<'static, str>, Vec<u8>)> {
		let joined = join_without_escape(&self.path, path)?;

		let mime = mime_guess::from_path(path).first_or_octet_stream();
		let mime = mime.essence_str();

		let content = tokio::fs::read(&joined)
			.await
			.with_context(|| format!("while reading {} as binary", path.display()))?;

		Ok((mime.to_string().into(), content))
	}
}
