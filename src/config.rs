use std::collections::HashMap;
use std::io::Write;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::fs::{File, read_dir};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::spawn;
use tokio::sync::{Mutex, RwLock, RwLockWriteGuard};
use tokio::task::{block_in_place, JoinHandle};
use tokio::time::{sleep, timeout};
use tracing::{debug, info, trace, warn};
use tracing::field::debug;
use zip::{CompressionMethod, ZipWriter};
use zip::write::FileOptions;

use crate::{config, RUNNING, status};
use crate::config::MinecraftServerStatus::{CRASHED, STARTING, STOPPED};
use crate::file_scanner::{scan_files, scan_files_exclude, scan_recursive};
use crate::minecraft_mod::MinecraftMod;
use crate::schema::{ForgeInfo, MinecraftServerConfig};

#[derive(Serialize, Deserialize)]
pub struct Config {
	pub(crate) minecraft: Minecraft,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			minecraft: Minecraft {
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
pub struct Minecraft {
	pub script: String,
	pub java: Option<String>,
	pub directory: String,
	#[serde(skip)]
	pub config_zip: RwLock<Vec<u8>>,
	pub(crate) folders: Vec<String>,
	pub(crate) exclude: String,
}

impl Minecraft {
	/// get path inside minecraft directory
	pub fn dir(&self, name: impl AsRef<Path>) -> PathBuf {
		Path::new(self.directory.as_str()).join(name)
	}

	pub async fn scan_mods(&self) -> Result<Vec<MinecraftMod>> {
		let mod_dir = self.dir("mods");
		if !mod_dir.exists() {
			return Ok(Vec::new());
		}
		let files = crate::file_scanner::scan_files_exclude(mod_dir, self.exclude.as_str()).await?;
		let mut file_infos = Vec::with_capacity(files.len());
		for x in files {
			file_infos.push(spawn(MinecraftMod::new(x)));
		}

		let f = join_all(file_infos).await.into_iter()
		                            .filter(|it| it.is_ok() && it.as_ref().unwrap().is_ok())
		                            .map(|it| it.unwrap().unwrap()).collect();
		Ok(f)
	}

	pub async fn zip_config(&self) -> Result<()> {
		let  mut content_handler = self.config_zip.write().await;
		let mut zip_content:Vec<u8> = std::mem::take(content_handler.as_mut());

		let mut out_zip = ZipWriter::new(std::io::Cursor::new(&mut zip_content));
		let option = FileOptions::default()
			.compression_method(CompressionMethod::Deflated)
			.large_file(false)
			.unix_permissions(0755);

		use futures::future;
		let configs = scan_recursive(self.dir("config"));
		let files: Vec<PathBuf> = configs
			.then(|ent| { future::ready(ent.ok().map(|it| it.path())) })
			.filter(|it| future::ready(it.is_some()))
			.map(|it| it.unwrap())
			.collect().await; // too complex to read if I continue to use stream

		let mut buffer = [0u8; 8192];
		let parent = self.dir("");
		for file in files {
			block_in_place(|| {
				Result::<()>::Ok(out_zip.start_file(file.strip_prefix(&parent)?.to_string_lossy(), option.clone())?)
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
		Ok(File::create(self.dir("current.json")).await?)
	}

	pub async fn current_config(&self) -> Option<MinecraftServerConfig> {
		let mut file = self.current_config_file().await.ok()?;
		if file.metadata().await.ok()?.len() == 0 {
			return None;
		}
		let mut str = String::new();
		str.reserve_exact(file.metadata().await.ok()?.len() as usize);
		file.read_to_string(&mut str).await.ok()?;
		Some(serde_json::from_str(&str).ok()?)
	}

	pub(crate) async fn spawn(&self) -> Result<Child> {
		debug!("Spawning server");
		let mut cmd = Command::new(self.dir(self.script.as_str()).canonicalize()?);
		let config = self.raw_config().await?.config;
		cmd.arg(format!("{}-{}", config.mc_version, config.forge_version));

		if let Some(java) = self.java.as_ref() {
			cmd.arg(java);
		}
		cmd.kill_on_drop(true);
		cmd.current_dir(self.dir("").canonicalize()?);
		cmd.stderr(Stdio::null());
		cmd.stdout(Stdio::piped());
		cmd.stdin(Stdio::piped());
		Ok(cmd.spawn()?)
	}

	pub async fn update_forge_cfg(&self, cfg: ForgeInfo) -> Result<MinecraftServerConfig> {
		let mut old_config = self.raw_config().await?;
		old_config.config = cfg;
		let cfg_json = serde_json::to_string_pretty(&old_config)?;
		let mut file = File::create(self.dir("config.json")).await?;
		file.write_all(cfg_json.as_bytes()).await?;
		file.shutdown().await?;
		Ok(self.scan_existing_mod(old_config).await?)
	}

	async fn raw_config(&self) -> Result<MinecraftServerConfig> {
		let mut file = File::open(self.dir("config.json")).await?;
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
		Ok(self.scan_existing_mod(config).await?)
	}

	pub async fn start(self) -> Result<MinecraftServer> {
		let process = self.spawn().await?;
		Ok(MinecraftServer::new(self, Some(process))?)
	}

	pub fn get_path(&self, path: &str) -> Option<PathBuf> {
		let path = path.trim_start_matches(&['/', '.']);
		let current = self.dir("").canonicalize().ok()?;
		let lookup = current.join(path).canonicalize().ok()?;
		Some(current.join(lookup.strip_prefix(&current).ok()?))
	}

	pub fn get_dir(&self, path: &str) -> Option<PathBuf> {
		return match self.get_path(path) {
			None => None,
			Some(path) => {
				if path.is_file() {
					Some(path.parent()?.to_path_buf())
				} else if path.exists() {
					Some(path)
				} else {
					None
				}
			}
		};
	}

	pub async fn list_dir(&self, path: &str) -> Vec<String> {
		let mut paths = Vec::new();

		let dir = match self.get_dir(path) {
			None => {
				return paths;
			}
			Some(dir) => {
				dir
			}
		};
		let current = if let Ok(path) = self.dir("").canonicalize() {
			path
		} else {
			return paths;
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

		paths
	}

	pub fn build(self) -> Result<MinecraftServer> {
		Ok(MinecraftServer::new(self, None)?)
	}
}

pub struct MinecraftServer {
	cfg: Minecraft,
	process: Arc<Mutex<Option<Child>>>,
	stdin: RwLock<Option<BufWriter<ChildStdin>>>,
	status: Arc<RwLock<MinecraftServerStatus>>,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum MinecraftServerStatus {
	STARTING,
	RUNNING,
	CRASHED,
	STOPPED,
}

impl Deref for MinecraftServer {
	type Target = Minecraft;

	fn deref(&self) -> &Self::Target {
		&self.cfg
	}
}

impl MinecraftServer {
	pub fn new(cfg: Minecraft, process: Option<Child>) -> Result<Self> {
		if let Some(mut process) = process {
			let stdout = process.stdout.take().unwrap();
			let stdin = BufWriter::new(process.stdin.take().unwrap());
			let status = Arc::new(RwLock::new(MinecraftServerStatus::STARTING));
			let status_clone = status.clone();

			let process = Arc::new(Mutex::new(Some(process)));
			let process_clone = process.clone();

			trace!("starting server");
			let this = Self { cfg, process, stdin: RwLock::new(Some(stdin)), status };
			this.create_heartbeat(stdout, status_clone, process_clone);
			Ok(this)
		} else {
			Ok(Self { cfg, process: Arc::new(Mutex::new(None)), stdin: RwLock::new(None), status: Arc::new(RwLock::new(MinecraftServerStatus::STOPPED)) })
		}
	}

	fn create_heartbeat(&self, stdout: ChildStdout, status: Arc<RwLock<MinecraftServerStatus>>, process_clone: Arc<Mutex<Option<Child>>>) {
		trace!("spawning heartbeat task");
		tokio::spawn(async move {
			{
				*status.write().await = MinecraftServerStatus::STARTING;
			}
			trace!("starting heartbeat");
			let mut stdout = BufReader::new(stdout).lines();
			trace!("waiting for output");

			while let Ok(Some(line)) = stdout.next_line().await {
				if line.contains(r#"For help, type "help""#) {
					info!("found help message; server started!");
					let mut s = status.write().await;
					*s = MinecraftServerStatus::RUNNING;
					break;
				}
			}

			debug!("reading output in background");
			while let Ok(Some(_)) = stdout.next_line().await {
				sleep(Duration::from_millis(20)).await;
			}

			debug!("Output stopped checking status");
			sleep(Duration::from_secs(1)).await;

			let mut process = process_clone.lock().await;
			if let Some(ref mut process) = *process {
				if let Ok(Ok(estatus)) = timeout(Duration::from_secs(2), process.wait()).await {
					let mut s = status.write().await;
					if estatus.success() {
						*s = MinecraftServerStatus::STOPPED;
					} else {
						warn!("Server crashed!");
						*s = MinecraftServerStatus::CRASHED;
					}
				}
			}

			()
		});
	}

	pub async fn status(&self) -> MinecraftServerStatus {
		*self.status.read().await
	}

	pub async fn scan_mods(&self) -> Result<Vec<MinecraftMod>> {
		self.cfg.scan_mods().await
	}

	pub async fn wait_started(&self) -> Result<()> {
		info!("waiting server to start");
		loop {
			let status = self.status().await;
			if status != MinecraftServerStatus::STARTING {
				break;
			}
			sleep(Duration::from_secs(2)).await;
		}
		Ok(())
	}

	pub async fn input(&self, message: impl AsRef<[u8]>) -> Result<()> {
		let data = message.as_ref();
		let mut stdin = self.stdin.write().await;
		if let Some(stdin) = stdin.as_mut() {
			stdin.write_all(data).await?;
			if let Some(b'\n') = data.last() {} else {
				stdin.write(&[b'\n']).await?;
			}
			stdin.flush().await?;
		}
		drop(stdin);

		Ok(())
	}

	pub async fn say(&self, message: impl AsRef<str>) -> Result<()> {
		let data = message.as_ref();
		let mut msg = String::with_capacity(5 + data.as_bytes().len());
		msg.push_str("say ");
		msg.push_str(data);
		msg.push('\n');
		let mut stdin = self.stdin.write().await;
		if let Some(stdin) = stdin.as_mut() {
			stdin.write_all(msg.as_bytes()).await?;
			stdin.flush().await?;
		}
		drop(stdin);
		Ok(())
	}

	pub async fn restart_in_place(&self) -> Result<()> {
		if self.status().await == MinecraftServerStatus::STARTING {
			warn!("Server is starting this restart will do nothing.");
			return Ok(());
		}
		self.shutdown_in_place().await.ok();
		let mut child = self.cfg.spawn().await?;
		if let Some(stdout) = child.stdout.take() {
			self.create_heartbeat(stdout, self.status.clone(), self.process.clone());
		}
		let stdin = child.stdin.take();
		*self.process.lock().await = Some(child);
		*self.stdin.write().await = stdin.map(|it| BufWriter::new(it));
		Ok(())
	}

	pub async fn update_config(&self) -> Result<MinecraftServerConfig> {
		Ok(self.create_config().await?)
	}

	async fn shutdown(&self, soft: bool) -> Result<()> {
		debug!("stopping server");
		let mut sin = self.stdin.write().await;
		trace!("taking stdin");
		let stdin = sin.take();
		drop(sin);
		let status = self.status().await;

		if let Some(mut stdin) = stdin {
			if status == RUNNING || status == STARTING {
				debug!("Server is running! stop event will wait for 15 seconds");
				stdin.write_all("say Server will stop within 15 seconds\n".as_bytes()).await?;
				stdin.flush().await?;
				let mut no = String::with_capacity(8);
				use std::fmt::Write;
				for counter in (0..=14).rev() {
					sleep(Duration::from_secs(1)).await;
					no.clear();
					no.push_str("say ");
					writeln!(&mut no, "{}", counter).ok();
					stdin.write_all(no.as_bytes()).await?;
					stdin.flush().await?;
				}
			}

			stdin.write_all("stop\n".as_bytes()).await?;
			stdin.flush().await?;
			sleep(Duration::from_secs(1)).await;
		} else {
			trace!("can't take stdin!");
		}

		if let Some(mut process) = self.process.lock().await.take() {
			if soft {
				process.wait().await?;
				process.kill().await?;
			} else {
				process.kill().await?;
			}
		}
		Ok(())
	}

	pub async fn kill(&self) -> Result<()> {
		self.shutdown(false).await
	}

	pub async fn shutdown_in_place(&self) -> Result<()> {
		self.shutdown(true).await
	}

	pub async fn stop(self) -> Result<Minecraft> {
		debug!("Sending stop command to server");
		self.shutdown_in_place().await?;
		info!("Server stopped");
		Ok(self.cfg)
	}
}