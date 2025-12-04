use anyhow::anyhow;

pub const PLACEHOLDER: &str = "{%body%}";

#[derive(Clone, Debug)]
pub struct Frame {
	template: String,
}
impl Default for Frame {
	fn default() -> Self {
		Self {
			template: format!("{PLACEHOLDER}"),
		}
	}
}

impl Frame {
	pub fn new(template: String) -> anyhow::Result<Self> {
		if !template.contains(PLACEHOLDER) {
			return Err(anyhow!(
				"template doesn't contain PLACEHOLDER {PLACEHOLDER}:\n\n{template}"
			));
		}

		Ok(Self { template })
	}

	pub fn with_child(&self, child: &Self) -> Self {
		let template = self.with_content(&child.template);
		Self { template }
	}
	pub fn with_parent(&self, parent: &Self) -> Self {
		parent.with_child(self)
	}

	pub fn with_content(&self, content: &str) -> String {
		self.template.replace(PLACEHOLDER, content)
	}
}
