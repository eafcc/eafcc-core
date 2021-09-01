use serde::{Deserialize, Serialize};
use serde_json;



#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResMeta {
    pub desc: String,
    pub tags: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResSpec(pub Vec<ResSpecItem>);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResSpecItem{
    pub content_type: String,
    pub key: String,
	pub data: String,
	pub schema: serde_json::Value,
}

