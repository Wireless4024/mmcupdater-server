pub trait ToJsonValue {
	/// Create json value from this object
	fn to_json(&self) -> serde_json::Value;
}