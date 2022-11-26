use std::io;
use std::io::{ErrorKind, Write};
use std::io::Result;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use futures::{StreamExt, TryFutureExt};
use pedestal_rs::fs::path;
use serde::{Deserialize, Serialize};
use tokio::fs::{File, metadata, read_dir};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::task::spawn_blocking;
use tracing::debug;
use zip::{CompressionMethod, ZipWriter};
use zip::result::ZipResult;
use zip::write::FileOptions;

use crate::file_scanner::scan_recursive;
use crate::util::errors::{sp_to_io, zip_to_io};
use crate::util::java::JavaManager;

static DEFAULT_JVM_ARGS: &str = include_str!("../resources/default_jvm_args.txt");

#[derive(Serialize, Deserialize, Clone)]
pub struct MinecraftConfig {
	//pub script: String,
	pub java: String,
	pub max_ram: u16,
	pub jvm_args: Vec<String>,
	pub server_file: String,
	pub args: Vec<String>,
	#[serde(skip)]
	pub directory: String,
	//pub(crate) exclude: String,
	//#[serde(skip_serializing_if = "Vec::is_empty")]
	pub(crate) dist_folder: Vec<String>,
}


impl Default for MinecraftConfig {
	fn default() -> Self {
		Self {
			java: String::new(),
			max_ram: 1024,
			jvm_args: DEFAULT_JVM_ARGS.split(char::is_whitespace)
				.filter(|it| !it.is_empty())
				.map(|it| it.to_string())
				.collect::<Vec<String>>(),
			server_file: "server.jar".to_string(),
			args: vec!["nogui".to_string()],
			directory: String::new(),
			dist_folder: vec![String::from("mods")],
			//exclude: String::from(".+\\.(bak|old)$"),
		}
	}
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

	pub async fn use_java(&mut self, java_id: &str) -> bool {
		let java = JavaManager::get_by_id(java_id).await;
		if let Some(java) = java {
			self.java = java.path_for(&self.directory).to_string_lossy().to_string();
			true
		} else {
			false
		}
	}

	/*

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
	}*/

	async fn zip_folder<W: Write + io::Seek + Send + 'static>(parent: &Path, folder: &Path, mut out_zip: ZipWriter<W>) -> Result<ZipWriter<W>> {
		use futures::future;
		let configs = scan_recursive(folder);
		let files: Vec<PathBuf> = configs
			.then(|ent| { future::ready(ent.ok().map(|it| it.path())) })
			.filter(|it| future::ready(it.is_some()))
			.map(|it| it.unwrap())
			.collect().await; // too complex to read if I continue to use stream
		let option = FileOptions::default()
			.compression_method(CompressionMethod::Deflated)
			.large_file(false)
			.unix_permissions(0o755);
		let mut buffer = vec![0u8; 8192];
		for file in files {
			let f = file.strip_prefix(&parent).map_err(sp_to_io)?.to_path_buf();
			out_zip = spawn_blocking(move || {
				out_zip.start_file(f.to_string_lossy(), option).map_err(zip_to_io)?;
				Result::<_>::Ok(out_zip)
			}).await??;

			let mut file_content = File::open(file).await?;
			loop {
				let read = file_content.read(&mut buffer).await?;
				match read {
					0 => { break; }
					n => {
						let (zip, buf) = spawn_blocking(move || {
							out_zip.write_all(&buffer[..n])?;
							Result::<_>::Ok((out_zip, buffer))
						}).await??;
						out_zip = zip;
						buffer = buf;
					}
				}
			}
		}
		Ok(out_zip)
	}

	pub async fn zip_dist(&self) -> Result<Vec<u8>> {
		let content_handler = Vec::<u8>::with_capacity(8192);
		let mut out_zip = ZipWriter::new(io::Cursor::new(content_handler));

		let parent = self.dir("")?;
		for x in &self.dist_folder {
			let path = self.dir(x)?;
			if metadata(&path).await.is_ok() {
				out_zip = Self::zip_folder(&parent, &path, out_zip).await?;
			}
		}
		let res: ZipResult<io::Cursor<Vec<u8>>> = spawn_blocking(move || {
			out_zip.finish()
		}).await?;

		Ok(res?.into_inner())
	}

	pub async fn zip_config(&self) -> Result<Vec<u8>> {
		let content_handler = Vec::<u8>::with_capacity(8192);
		let mut out_zip = ZipWriter::new(io::Cursor::new(content_handler));

		let parent = self.dir("")?;
		let path = self.dir("config")?;
		if metadata(&path).await.is_ok() {
			out_zip = Self::zip_folder(&parent, &path, out_zip).await?;
		}
		let res: ZipResult<io::Cursor<Vec<u8>>> = spawn_blocking(move || {
			out_zip.finish()
		}).await?;

		Ok(res?.into_inner())
	}

	pub async fn current_config_file(&self) -> Result<File> {
		File::create(self.dir("current.json")?).await
	}/*

	pub async fn current_config(&self) -> Option<MinecraftServerConfig> {
		let mut file = self.current_config_file().await.ok()?;
		if file.metadata().await.ok()?.len() == 0 {
			return None;
		}
		let mut str = String::new();
		str.reserve_exact(file.metadata().await.ok()?.len() as usize);
		file.read_to_string(&mut str).await.ok()?;
		serde_json::from_str(&str).ok()
	}*/

	pub(crate) async fn spawn(&self) -> io::Result<Child> {
		debug!("Spawning server");
		let mut cmd = Command::new(&self.java);
		cmd.args(&self.jvm_args);
		cmd.arg(format!("-Xmx{}M", self.max_ram));
		cmd.arg("-jar");
		cmd.arg(&self.server_file);
		cmd.args(&self.args);
		cmd.kill_on_drop(true);
		cmd.current_dir(self.canonicalized("")?);
		cmd.stderr(Stdio::inherit());
		cmd.stdout(Stdio::piped());
		cmd.stdin(Stdio::piped());
		cmd.spawn()
	}/*

	pub async fn update_forge_cfg(&self, cfg: ForgeInfo) -> Result<MinecraftServerConfig> {
		let mut old_config = self.raw_config().await?;
		old_config.config = cfg;
		let cfg_json = serde_json::to_string_pretty(&old_config)?;
		let mut file = File::create(self.dir("config.json")?).await?;
		file.write_all(cfg_json.as_bytes()).await?;
		file.shutdown().await?;
		self.scan_existing_mod(old_config).await
	}*/

	/*async fn scan_existing_mod(&self, mut config: MinecraftServerConfig) -> Result<MinecraftServerConfig> {
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
	}*/
	/*
		pub async fn create_config(&self) -> Result<MinecraftServerConfig> {
			let config = self.raw_config().await?;
			self.scan_existing_mod(config).await
		}*/

	/*pub async fn start(self) -> Result<MinecraftServer> {
		let process = self.spawn().await?;
		MinecraftServer::new(,Some(process))
	}*/

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
		let current = if let Ok(path) = self.canonicalized("") {
			path
		} else {
			return Ok(paths);
		};

		let mut files = read_dir(dir).await.unwrap();
		while let Ok(Some(entry)) = files.next_entry().await {
			let p = entry.path();
			let is_dir = metadata(&p).await?.is_dir();
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

	/*pub fn build(self) -> Result<MinecraftServer> {
		MinecraftServer::new(None)
	}*/
}