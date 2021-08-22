use serde::{Serialize, Deserialize};
use serde_json;
use crate::rule_engine::{Rule, RuleMeta, RuleSpec};
use crate::error::DataLoaderError;
use super::object::ObjectID;

use std::collections::BTreeMap;


#[derive(Serialize, Deserialize)]
pub struct LinkSpec {
	pub pri: f32,
	pub is_neg: bool,
	pub ver: String,
	pub rule: String,
	pub reses: Vec<String>,
}

pub struct LinkTarget {
	target: Vec<ObjectID>,
	pri: f32,
	is_neg: bool,
}

pub struct LinkStorage{
	idx_rule_to_res: BTreeMap<String, LinkTarget>,

}

impl LinkStorage {

}

// pub load_link() {

// }