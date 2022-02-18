use std::collections::HashMap;
use std::fs::create_dir_all;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::AsyncReadExt;
use futures::future::join_all;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt as OtherAsyncReadExt, AsyncWriteExt, BufReader, BufWriter, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::spawn;
use tokio::sync::{Mutex, RwLock, RwLockWriteGuard};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tracing::{debug, info, trace, warn};
use tracing::field::debug;

use crate::{config, RUNNING, status};
use crate::config::MinecraftServerStatus::{CRASHED, STARTING, STOPPED};
use crate::file_scanner::scan_files_exclude;
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

	pub fn download_dir(&self) -> PathBuf {
		let dir = self.dir("downloads");
		if !dir.exists() {
			create_dir_all(&dir).ok();
		}
		dir
	}

	pub async fn update_config(&self) -> Result<MinecraftServerConfig> {
		/*let recent = self.create_config().await?;

		match self.current_config().await {
			Some(current) => {
				let (to_add, to_remove) = current.diff_mod(&recent);
				println!("ADD {:?}", to_add);
				println!("REMOVE {:?}", to_remove);
			}
			None => {
				let  mut file = self.current_config_file().await?;
				let  json = serde_json::to_string(&recent)?;
				file.write_all(json.as_bytes()).await?;
				file.shutdown().await?;
			}
		};
*/
		todo!()
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
	pub fn new(cfg: Minecraft, mut process: Option<Child>) -> Result<Self> {
		if let Some(mut process) = process {
			let stdout = process.stdout.take().unwrap();
			let stdin = BufWriter::new(process.stdin.take().unwrap());
			let status = Arc::new(RwLock::new(MinecraftServerStatus::STARTING));
			let status_clone = status.clone();

			let mut process = Arc::new(Mutex::new(Some(process)));
			let mut process_clone = process.clone();

			trace!("starting server");
			let this = Self { cfg, process, stdin: RwLock::new(Some(stdin)), status };
			this.create_heartbeat(stdout, status_clone, process_clone);
			Ok(this)
		} else {
			Ok(Self { cfg, process: Arc::new(Mutex::new(None)), stdin: RwLock::new(None), status: Arc::new(RwLock::new(MinecraftServerStatus::STOPPED)) })
		}
	}

	fn create_heartbeat(&self, mut stdout: ChildStdout, status: Arc<RwLock<MinecraftServerStatus>>, process_clone: Arc<Mutex<Option<Child>>>) {
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
		let mut stdin = sin.take();
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

	pub async fn stop(mut self) -> Result<Minecraft> {
		debug!("Sending stop command to server");
		self.shutdown_in_place().await?;
		info!("Server stopped");
		Ok(self.cfg)
	}
}