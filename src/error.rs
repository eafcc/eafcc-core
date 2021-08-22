use serde_json;
use thiserror::Error;

// 顶级Error
#[derive(Error, Debug)]
pub enum CCLibError {
	
}


#[derive(Error, Debug)]
pub enum DataLoaderError {
	#[error("error when parsing config data")]
	DataParseError{
		#[from]
		source: serde_json::Error,
	},
	#[error("error parse `spec` part in config: {0}")]
	SpecParseError(String),
}

