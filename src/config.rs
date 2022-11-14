use std::collections::HashMap;
use std::io;
use std::io::{ErrorKind, Write};
use std::io::Result;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use futures::future::join_all;
use futures::StreamExt;
use pedestal_rs::fs::path;
use serde::{Deserialize, Serialize};
use tokio::fs::{File, read_dir};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::spawn;
use tokio::sync::RwLock;
use tokio::task::block_in_place;
use tracing::debug;
use zip::{CompressionMethod, ZipWriter};
use zip::write::FileOptions;

use crate::file_scanner::scan_recursive;
use crate::instance::mc_mod::MinecraftMod;
use crate::instance::mc_server::MinecraftServer;
use crate::schema::{ForgeInfo, MinecraftServerConfig};

#[derive(Serialize, Deserialize)]
pub struct Config {
	pub(crate) minecraft: MinecraftConfig,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			minecraft: MinecraftConfig {
				script: String::from("start.sh"),
				java: None,
				directory: String::from("mc"),
				config_zip: RwLock::new(Vec::new()),
				folders: vec![String::from("mods")],
				exclude: String::from(".+\\.(bak|old)$"),
			}
		}
	}
}

#[derive(Serialize, Deserialize)]
pub struct MinecraftConfig {
	pub script: String,
	pub java: Option<String>,
	pub directory: String,
	#[serde(skip)]
	pub config_zip: RwLock<Vec<u8>>,
	pub(crate) folders: Vec<String>,
	pub(crate) exclude: String,
}

impl MinecraftConfig {
	/// get path inside minecraft directory
	pub fn dir(&self, name: impl AsRef<Path>) -> Result<PathBuf> {
		path::normalize(self.directory.as_ref(), name)
	}
	/// get canonicalize path inside minecraft directory
	pub fn canonicalized(&self, name: impl AsRef<Path>) -> Result<PathBuf> {
		let path: &Path = self.directory.as_ref();
		path::normalize(&path.canonicalize()?, name)
	}

	pub async fn scan_mods(&self) -> Result<Vec<MinecraftMod>> {
		let mod_dir = self.dir("mods")?;
		if !mod_dir.exists() {
			return Ok(Vec::new());
		}
		let files = crate::file_scanner::scan_files_exclude(mod_dir, self.exclude.as_str()).await?;
		let mut file_infos = Vec::with_capacity(files.len());
		for x in files {
			file_infos.push(spawn(MinecraftMod::try_parse(x)));
		}

		let f = join_all(file_infos).await.into_iter()
			.filter(|it| it.is_ok() && it.as_ref().unwrap().is_ok())
			.map(|it| it.unwrap().unwrap()).collect();
		Ok(f)
	}

	pub async fn zip_config(&self) -> Result<()> {
		let mut content_handler = self.config_zip.write().await;
		let mut zip_content: Vec<u8> = std::mem::take(content_handler.as_mut());

		let mut out_zip = ZipWriter::new(std::io::Cursor::new(&mut zip_content));
		let option = FileOptions::default()
			.compression_method(CompressionMethod::Deflated)
			.large_file(false)
			.unix_permissions(0o755);

		use futures::future;
		let configs = scan_recursive(self.dir("config")?);
		let files: Vec<PathBuf> = configs
			.then(|ent| { future::ready(ent.ok().map(|it| it.path())) })
			.filter(|it| future::ready(it.is_some()))
			.map(|it| it.unwrap())
			.collect().await; // too complex to read if I continue to use stream

		let mut buffer = [0u8; 8192];
		let parent = self.dir("")?;
		for file in files {
			block_in_place(|| {
				let file = match file.strip_prefix(&parent) {
					Ok(p) => {
						p
					}
					Err(e) => {
						return Err(io::Error::new(ErrorKind::Unsupported, e));
					}
				};
				Result::<()>::Ok(out_zip.start_file(file.to_string_lossy(), option)?)
			})?;
			let mut file_content = File::open(file).await?;
			while let Ok(read) = file_content.read(&mut buffer).await {
				match read {
					0 => {
						break;
					}
					n => {
						block_in_place(|| {
							out_zip.write_all(&buffer[..n])
						})?;
					}
				}
			}
		}
		block_in_place(|| {
			out_zip.finish()
		})?;
		drop(out_zip);
		zip_content.truncate(zip_content.len());
		*content_handler = zip_content;

		Ok(())
	}

	pub async fn get_config_zip(&self) -> Result<Vec<u8>> {
		let content = self.config_zip.read().await;
		if content.is_empty() {
			drop(content);
			self.zip_config().await?;
			Ok(Vec::clone(&*self.config_zip.read().await))
		} else {
			Ok(Vec::clone(&*content))
		}
	}

	pub async fn current_config_file(&self) -> Result<File> {
		File::create(self.dir("current.json")?).await
	}

	pub async fn current_config(&self) -> Option<MinecraftServerConfig> {
		let mut file = self.current_config_file().await.ok()?;
		if file.metadata().await.ok()?.len() == 0 {
			return None;
		}
		let mut str = String::new();
		str.reserve_exact(file.metadata().await.ok()?.len() as usize);
		file.read_to_string(&mut str).await.ok()?;
		serde_json::from_str(&str).ok()
	}

	pub(crate) async fn spawn(&self) -> io::Result<Child> {
		debug!("Spawning server");
		let mut cmd = Command::new(self.canonicalized(self.script.as_str())?);
		let config = self.raw_config().await?.config;
		cmd.arg(format!("{}-{}", config.mc_version, config.forge_version));

		if let Some(java) = self.java.as_ref() {
			cmd.arg(java);
		}
		cmd.kill_on_drop(true);
		cmd.current_dir(self.canonicalized("")?);
		cmd.stderr(Stdio::null());
		cmd.stdout(Stdio::piped());
		cmd.stdin(Stdio::piped());
		cmd.spawn()
	}

	pub async fn update_forge_cfg(&self, cfg: ForgeInfo) -> Result<MinecraftServerConfig> {
		let mut old_config = self.raw_config().await?;
		old_config.config = cfg;
		let cfg_json = serde_json::to_string_pretty(&old_config)?;
		let mut file = File::create(self.dir("config.json")?).await?;
		file.write_all(cfg_json.as_bytes()).await?;
		file.shutdown().await?;
		self.scan_existing_mod(old_config).await
	}

	async fn raw_config(&self) -> Result<MinecraftServerConfig> {
		let mut file = File::open(self.dir("config.json")?).await?;
		let mut data = String::new();
		file.read_to_string(&mut data).await?;
		let config: MinecraftServerConfig = serde_json::from_str(data.as_str())?;
		Ok(config)
	}

	async fn scan_existing_mod(&self, mut config: MinecraftServerConfig) -> Result<MinecraftServerConfig> {
		let mut cfg = self.config_zip.write().await;
		cfg.clear();
		drop(cfg);
		let mut mod_table = HashMap::<String, MinecraftMod>::new();
		for mc_mod in std::mem::take(&mut config.mods).into_iter() {
			mod_table.insert(mc_mod.name.to_string(), mc_mod);
		};

		let loaded_mod = self.scan_mods().await?;
		for mc_mod in loaded_mod.into_iter() {
			if !mod_table.contains_key(mc_mod.name.as_str()) {
				mod_table.insert(mc_mod.name.to_string(), mc_mod);
			}
		}

		config.mods = mod_table.into_values().collect();
		Ok(config)
	}

	pub async fn create_config(&self) -> Result<MinecraftServerConfig> {
		let config = self.raw_config().await?;
		self.scan_existing_mod(config).await
	}

	pub async fn start(self) -> Result<MinecraftServer> {
		let process = self.spawn().await?;
		MinecraftServer::new(Some(process))
	}

	pub fn get_dir(&self, path: &str) -> Result<PathBuf> {
		let path = self.dir(path)?;
		if path.is_file() {
			Ok(path.parent().ok_or_else(|| io::Error::from(ErrorKind::NotADirectory))?.to_path_buf())
		} else if path.exists() {
			Ok(path)
		} else {
			Err(ErrorKind::NotADirectory.into())
		}
	}

	pub async fn list_dir(&self, path: &str) -> Result<Vec<String>> {
		let mut paths = Vec::new();

		let dir = match self.get_dir(path) {
			Err(_) => {
				return Ok(paths);
			}
			Ok(dir) => {
				dir
			}
		};
		let current = if let Ok(path) = self.canonicalized(""){
			path
		} else {
			return Ok(paths);
		};

		let mut files = read_dir(dir).await.unwrap();
		while let Ok(Some(entry)) = files.next_entry().await {
			let p = entry.path();
			let is_dir = p.is_dir();
			if let Ok(path) = p.strip_prefix(&current) {
				let mut p = path.to_string_lossy().to_string();
				if is_dir {
					p.reserve_exact(1);
					p.push('/');
				}
				paths.push(p);
			}
		}

		Ok(paths)
	}

	pub fn build(self) -> Result<MinecraftServer> {
		MinecraftServer::new(None)
	}
}