use serde::{Serialize, Deserialize};
use serde_json;
use crate::rule_engine::{Rule, RuleMeta, RuleSpec};
use crate::error::DataLoaderError;


#[derive(Serialize, Deserialize)]
pub struct RootCommon {
	pub version: u32,
	pub kind: String,
	pub meta: serde_json::Value,
	pub spec: serde_json::Value,
}


mod rule;
mod link;
mod object;
mod res;