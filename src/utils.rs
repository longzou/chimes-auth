
use std::fmt::{Debug};
use std::time::{SystemTime};
use serde_derive::{Deserialize, Serialize};
use chrono::offset::Local;
use chrono::DateTime;

pub fn get_local_timestamp() -> u64 {
    let now = SystemTime::now();
    let date:DateTime<Local> = now.into();
    date.timestamp_millis() as u64
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct  ApiResult <T> {
    pub status: i32,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: Option<u64>,
}

impl<T> ApiResult<T> {

    pub fn ok (dt: T) -> Self {
        ApiResult {
            status: 200,
            message: "OK".to_string(),
            data: Option::Some(dt),
            timestamp: Some(get_local_timestamp())
        }
    }

    pub fn error (code: i32, msg: &String) -> Self {
        ApiResult {
            status: code,
            message: msg.to_owned(),
            data: None,
            timestamp: Some(get_local_timestamp())
        }
    }

    pub fn new (code: i32, msg: &String, data: T, ts: u64) -> Self {
        ApiResult {
            status: code,
            message: msg.to_owned(),
            data: Some(data),
            timestamp: Some(ts)
        }
    }
}

