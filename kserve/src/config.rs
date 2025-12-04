use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	allow_indexing: bool,
}
impl Default for Config {
	fn default() -> Self {
		Config {
			allow_indexing: true,
		}
	}
}
