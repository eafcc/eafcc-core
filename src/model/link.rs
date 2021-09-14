use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::DataLoaderError;

use super::{RootCommon, object::ObjectID};

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


pub struct Link {
    pub meta: LinkMeta,
    pub spec: LinkSpec,
}


impl Link {
    pub fn load_from_slice(link_data: &[u8]) -> Result<Link, DataLoaderError> {
        let root = serde_json::from_slice::<RootCommon>(link_data)?;
        let meta = serde_json::from_value::<LinkMeta>(root.meta)?;
        let spec = serde_json::from_value::<LinkSpec>(root.spec)?;
        if !spec.pri.is_finite() {
            return Err(DataLoaderError::SpecParseError(
                "pri field is not a valid float number".to_string(),
            ));
        }
        return Ok(Link{meta, spec})
    }
}