use std::path::Path;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use crate::get_manifest;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MinecraftMod {
	pub name: String,
	pub version: String,
	pub file_name: String,
}

impl MinecraftMod {
	pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
		let path = path.as_ref();
		let manifest = get_manifest(path).await?;
		let file_name = path.file_name().map(|it| it.to_string_lossy()).unwrap_or_default();
		Ok(Self {
			name: manifest.get("Implementation-Title").map(|it| it.to_string()).unwrap_or_else(|| {
				let file_name = file_name.splitn(2, "-").next();
				file_name.unwrap_or_default().to_string()
			}).to_string(),
			version: manifest.get("Implementation-Version").map(|it| it.to_string()).unwrap_or_else(|| {
				let mut part = file_name.splitn(2, "-");
				part.next();
				let version = part.next();
				version.unwrap_or_default().to_string()
			}).to_string(),
			file_name: file_name.to_string(),
		})
	}
}