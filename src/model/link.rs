use serde::{Serialize, Deserialize};
use serde_json;
use crate::rule_engine::{Rule, RuleMeta, RuleSpec};
use crate::error::DataLoaderError;
use super::object::ObjectID;

use std::collections::BTreeMap;
use super::RootCommon;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct LinkMeta {
    pub desc: String,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LinkSpec {
	pub pri: f32,
	pub is_neg: bool,
	pub ver: String,
	pub rule: String,
	pub reses: Vec<String>,
}

pub struct LinkTarget {
	pub target: Vec<ObjectID>,
	pub pri: f32,
	pub is_neg: bool,
	pub link_id: String, 
}

