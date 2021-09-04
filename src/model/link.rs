use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::object::ObjectID;

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
    #[serde(rename = "res")]
    pub reses: Vec<String>,
}


pub struct LinkInfo {
    pub reses_path: Vec<String>,
    pub pri: f32,
    pub is_neg: bool,
    pub link_path: Arc<String>,
	pub rule_path: Arc<String>,
}
