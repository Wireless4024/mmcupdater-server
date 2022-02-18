use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::{BufRead, BufReader, Lines};
use std::iter::Map;
use std::ops::Deref;
use std::path::Path;
use anyhow::{bail, Result};
use zip::read::ZipFile;
use zip::ZipArchive;

pub async fn get_manifest(file: impl AsRef<Path>) -> Result<JarManifest> {
	let mut file = File::open(file)?;
	let mut zip = if let Ok(zip) = ZipArchive::new(&mut file) {
		zip
	} else {
		bail!("Failed to open zip");
	};
	let entry = zip.by_name("META-INF/MANIFEST.MF")?;
	let mut reader = BufReader::new(entry);
	let mut lines = reader.lines();
	Ok(JarManifest::new(&mut lines)?)
}

#[derive(Debug)]
pub struct JarManifest(HashMap<String, String>);

impl Deref for JarManifest {
	type Target = HashMap<String, String>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl JarManifest {
	pub fn new(lines: &mut Lines<BufReader<ZipFile>>) -> Result<Self> {
		let mut attr = HashMap::new();
		while let Some(Ok(line)) = lines.next() {
			let mut split = line.splitn(2, ":");
			if let Some(key) = split.next() {
				attr.insert(key.to_string(), split.as_str().trim().to_string());
			}
		}
		Ok(Self(attr))
	}
}