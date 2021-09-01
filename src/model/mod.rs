use serde::{Serialize, Deserialize};
use serde_json;




#[derive(Serialize, Deserialize)]
pub struct RootCommon {
	pub version: u32,
	pub kind: String,
	pub meta: serde_json::Value,
	pub spec: serde_json::Value,
}


pub mod rule;
pub mod link;
pub mod object;
pub mod res;