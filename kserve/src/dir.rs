use std::path::{Component, Path, PathBuf};

use anyhow::Context;

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
	let b = b.with_context(|| {
		format!(
			"failed to normalize {} without it leaking into the outer fs",
			untrusted_b.display()
		)
	})?;

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

	pub async fn md_to_html(&self, path: &Path) -> anyhow::Result<String> {
		let joined = join_without_escape(&self.path, path)?;

		let md = tokio::fs::read_to_string(&joined)
			.await
			.with_context(|| format!("while reading {}", path.display()))?;

		let opts = comrak::Options::default();
		let html = comrak::markdown_to_html(&md, &opts);

		Ok(html)
	}
}
