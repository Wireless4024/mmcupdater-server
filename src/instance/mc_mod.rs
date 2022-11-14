use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::get_manifest;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MinecraftMod {
	pub name: String,
	pub version: String,
	pub file_name: String,
}

impl MinecraftMod {
	pub async fn try_parse(path: impl AsRef<Path>) -> Result<Self> {
		let path = path.as_ref();
		let manifest = get_manifest(path).await?;
		let file_name = path.file_name().map(|it| it.to_string_lossy()).unwrap_or_default();

		let name = manifest
			.get("Specification-Title")
			.map(|it| it.to_string())
			.unwrap_or_else(|| {
				manifest
					.get("Implementation-Title")
					.map(|it| it.to_string())
					.unwrap_or_else(|| {
						let file_name = file_name.split('-').next();
						file_name.unwrap_or_default().to_string()
					})
			});
		Ok(Self {
			name,
			version: manifest.get("Implementation-Version").map(|it| it.to_string()).unwrap_or_else(|| {
				let mut part = file_name.splitn(2, '-');
				part.next();
				let version = part.next();
				version.unwrap_or_default().to_string()
			}),
			file_name: file_name.to_string(),
		})
	}

	pub fn diff_mod<'a, 'b>(old: &'a Vec<MinecraftMod>, recent: &'b Vec<MinecraftMod>) -> ModDiff<'b, 'a> {
		let mut to_add = Vec::new();
		let mut existing = HashMap::new();

		for mc_mod in old {
			existing.insert(&mc_mod.name, mc_mod);
		}

		for x in recent {
			if let Some(m) = existing.get(&x.name) {
				existing.remove(&m.name);
			} else {
				to_add.push(x);
			}
		}

		ModDiff { to_add, to_remove: existing.into_values().collect() }
	}
}

pub struct ModDiff<'a, 'b> {
	to_add: Vec<&'a MinecraftMod>,
	to_remove: Vec<&'b MinecraftMod>,
}