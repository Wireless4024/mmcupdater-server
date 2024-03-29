use std::time::{SystemTime, UNIX_EPOCH};

pub fn timestamp_minute() -> u64 {
	SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() / 60
}