use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResMeta {
    pub name: String,
    pub desc: String,
    pub tags: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct ResSpec {
    pub content_type: String,
	pub data: String,
	pub schema: serde_json::Value,
}