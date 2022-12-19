pub trait ToJsonValue {
	fn to_json(&self) -> serde_json::Value;
}