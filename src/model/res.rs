use serde::{Deserialize, Serialize};
use serde_json;

use crate::error::DataLoaderError;

use super::RootCommon;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResMeta {
    pub desc: String,
    pub tags: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResSpec(pub Vec<ResSpecItem>);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResSpecItem {
    pub content_type: String,
    pub key: String,
    pub data: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, PartialEq)]
pub struct Res {
    pub meta: ResMeta,
    pub spec: ResSpec,
}

impl Res {
    pub fn load_from_slice(res_data: &[u8]) -> Result<Res, DataLoaderError> {
        let root = serde_json::from_slice::<RootCommon>(res_data)?;
        let meta = serde_json::from_value::<ResMeta>(root.meta)?;
        let spec = serde_json::from_value::<ResSpec>(root.spec)?;
        return Ok(Res { meta, spec });
    }
}
