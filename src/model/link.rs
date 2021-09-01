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

pub struct LinkTarget {
    pub target: Vec<ObjectID>,
    pub pri: f32,
    pub is_neg: bool,
    pub link_id: String,
}
