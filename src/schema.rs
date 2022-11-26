use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::instance::mc_mod::MinecraftMod;
/*
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MinecraftServerConfig {
	pub config: ForgeInfo,
	#[serde(default)]
	pub mods: Vec<MinecraftMod>,
}

impl MinecraftServerConfig {
	pub fn diff_mod(&self, recent: &Self) -> (Vec<MinecraftMod>, Vec<MinecraftMod>) {
		let mut to_add = recent.mods.clone();
		let mut existing = HashMap::new();

		for mc_mod in &self.mods {
			existing.insert(mc_mod.name.as_str(), mc_mod);
		}

		to_add.retain(|it| {
			if let Some(m) = existing.get(it.name.as_str()) {
				let name_str = m.name.as_str();
				existing.remove(name_str);
				false
			} else {
				true
			}
		});

		(to_add, existing.into_values().map(|it| it.clone()).collect())
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ForgeInfo {
	pub mc_version: String,
	pub forge_version: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub identifier: Option<String>,
}*/