use std::cmp::Ordering;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::{extract, Json};
use axum::body::{Body, BoxBody, boxed};
use axum::http::{Request, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use hashbrown::HashMap;
use pedestal_rs::ext::ArcExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::fs::{create_dir_all, File, metadata, read_dir, remove_dir_all, remove_file, rename};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::spawn;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::instance::mc_mod::MinecraftMod;
use crate::instance::mc_server::MinecraftServer;
use crate::instance::mc_server::MinecraftServerStatus::STOPPED;
use crate::mc::mc_config::MinecraftConfig;
use crate::mc::mc_version::java_for;
use crate::util::errors::reqwest_to_io;
use crate::util::fs::{create_if_not_existed, OwnedDirEntry};
use crate::util::http::{download_to, new_client};
use crate::util::java::JavaManager;

static CONFIG_DOCS: &str = include_str!("../resources/config_docs.yml");

#[derive(Serialize, Deserialize, Clone)]
pub struct McInstance {
	/// Instance name (may generated from folder name)
	#[serde(default)]
	pub name: String,
	/// Minecraft version
	version: String,
	/// Minecraft configuration  
	/// ### Note 
	/// to change its field replace instead
	pub config: Arc<MinecraftConfig>,
	/// Type of modded server
	#[serde(default)]
	pub mod_type: ModType,
	/// Used to store server process
	#[serde(skip)]
	_server_instance: Option<Arc<MinecraftServer>>,
	/// List of mods
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub mods: Vec<MinecraftMod>,
}

impl Default for McInstance {
	fn default() -> Self {
		Self {
			name: String::new(),
			version: String::new(),
			config: Arc::new(MinecraftConfig::default()),
			mod_type: Default::default(),
			mods: vec![],
			_server_instance: None,
		}
	}
}

impl McInstance {
	pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
		let path = path.as_ref();
		let config = path.join("config.yml");
		let mut cfg_file = match File::open(&config).await {
			Ok(f) => { f }
			Err(_) => {
				return Self::generate(path, "", ModType::default()).await;
			}
		};
		let mut data = Vec::new();
		cfg_file.shutdown().await?;
		cfg_file.read_to_end(&mut data).await?;
		let cfg = serde_yaml::from_slice::<Self>(&data);
		if let Ok(mut config) = cfg {
			if config.name.is_empty() {
				config.name = path.file_name().map(|it| it.to_string_lossy().to_string()).expect("Instance name");
			}
			Arc::get_mut(&mut config.config).unwrap().directory = path.canonicalize()?.to_string_lossy().to_string();
			Ok(config)
		} else {
			Self::generate(path, "", ModType::default()).await
		}
	}

	/// Attempt to generate new instance provided folder.  
	/// It will create blank config and save to instance location.
	///
	/// # Arguments 
	///
	/// * `folder`: Instance folder (will replace existing)
	pub async fn generate(folder: &Path, version: &str, mod_type: ModType) -> Result<McInstance> {
		let mut cfg = Self::default();
		#[allow(clippy::field_reassign_with_default)]
		{ cfg.mod_type = mod_type; }
		cfg.version = version.to_string();
		if cfg.name.is_empty() {
			cfg.name = folder.file_name().map(|it| it.to_string_lossy().to_string()).expect("Instance name");
		}
		if cfg.version.is_empty() {
			match &mut cfg.mod_type {
				ModType::Vanilla | ModType::Purpur => {
					let versions = ModType::Purpur.versions(&new_client()?).await?;
					match versions.latest() {
						None => {}
						Some(ver) => {
							cfg.version = ver.to_string();
						}
					}
				}
				n => {
					// use to generate jump instruction
					#[allow(clippy::never_loop)]
					loop {
						if let ModType::Forge(ver) = n {
							if !ver.is_empty() {
								// jump out
								break;
							}
						}
						let versions = n.versions(&new_client()?).await?;
						match versions.latest() {
							None => {}
							Some(ver) => {
								cfg.version = ver.to_string();
								if let Some(it) = versions.latest_for(ver) {
									if let ModType::Forge(fver) = n {
										*fver = it.to_string();
									}
								}
							}
						}
						break;
					}
				}
			}
		}
		{
			let java = java_for(&cfg.version).unwrap();
			let folder = folder.to_string_lossy().to_string();
			cfg.config.modify_async(|c| Box::pin(async {
				c.directory = folder;
				if c.java.is_empty() {
					let j = JavaManager::get_version(java.recommended).await;
					match j {
						None => {
							c.java = "java".to_string();
						}
						Some(it) => {
							c.java = it.path_for(&c.directory).to_string_lossy().to_string();
							let extra_args = it.performance_args();
							if !extra_args.is_empty()
								&& !c.jvm_args.iter().any(|it| it == extra_args[0]) {
								c.jvm_args.extend(extra_args.iter().map(|it| it.to_string()));
							}
						}
					}
				}
			})).await;
		}
		cfg.save().await?;
		Ok(cfg)
	}

	// try to initialize instance (download server file as needed)
	pub async fn init(&mut self) -> Result<()> {
		let cfg = &self.config;
		// server file is not existed
		let server_path = self.dir(&cfg.server_file)?;
		if metadata(&server_path).await.is_err() {
			// it will fail to init if failed to download
			self.mod_type.download_server(&new_client()?, &self.version, server_path).await?;
		};
		create_if_not_existed(self.dir("eula.txt")?, b"eula=true").await?;
		if self._server_instance.is_none() {
			self._server_instance = Some(Arc::new(MinecraftServer::new(self.name.clone(), None)?));
		}
		Ok(())
	}

	pub async fn get_file(&self, path: impl AsRef<Path>) -> Option<File> {
		let path = path.as_ref();
		let file = self.dir(path).ok()?;
		// existed
		if metadata(&file).await.is_ok() {
			File::open(file).await.ok()
		} else {
			None
		}
	}

	pub async fn list_dir(&self, path: impl AsRef<Path>) -> Result<Vec<OwnedDirEntry>> {
		let path = path.as_ref();
		let parent = self.dir("")?;
		let file = self.dir(path)?;
		// existed
		if let Ok(meta) = metadata(&file).await {
			// umm we can't list dir from file :(
			if meta.is_file() {
				return Ok(Vec::new());
			}
			let mut entry = Vec::new();
			let mut dir = read_dir(file).await?;

			while let Some(ent) = dir.next_entry().await? {
				let metadata = ent.metadata().await?;
				entry.push(OwnedDirEntry {
					is_dir: metadata.is_dir(),
					name: ent.path().strip_prefix(&parent)
						.map(|it| it.as_os_str().to_os_string())
						.unwrap_or_else(|_| ent.path().into_os_string()),
				});
			}
			Ok(entry)
		} else {
			Ok(Vec::new())
		}
	}

	async fn get_web_file(&self, uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
		let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
		match ServeDir::new(&self.config.directory).oneshot(req).await {
			Ok(res) => Ok(res.map(boxed)),
			Err(err) => Err(
				(
					StatusCode::INTERNAL_SERVER_ERROR,
					format!("Something went wrong: {}", err)
				)
			),
		}
	}


	/// Serialize configuration to file, config file will stored in instance folder (self.config.directory)
	pub async fn save(&self) -> Result<()> {
		let data = serde_yaml::to_string(self).expect("Serialize config");
		let mut cfg_file = File::create((AsRef::<Path>::as_ref(&self.config.directory)).join("config.yml")).await?;
		cfg_file.write_all(data.as_bytes()).await?;
		cfg_file.write_all(CONFIG_DOCS.as_bytes()).await?;
		cfg_file.flush().await?;
		cfg_file.shutdown().await?;
		Ok(())
	}

	fn update_config(&mut self) {}

	pub async fn update(&mut self, mut multipart: extract::Multipart) -> impl IntoResponse {
		while let Ok(Some(field)) = multipart.next_field().await {
			let name = field.name().unwrap();
			if let "file" = name {
				let filename = field.file_name()
					.and_then(|it| it.rsplit('/').next())
					.map(|it| it.to_string());
				if let Some(true) = filename.as_ref().map(|it| it.ends_with(".jar")) {
					let filename = filename.unwrap();

					let dl_folder = Path::new("download");
					if !dl_folder.exists() {
						create_dir_all(&dl_folder).await.unwrap();
					}
					let tmp = dl_folder.join(filename.as_str());

					let mut tmp_file = File::create(&tmp).await.unwrap();

					let mut reader = field;
					while let Some(Ok(data)) = reader.next().await {
						tmp_file.write_all(&data).await.ok();
					};
					tmp_file.flush().await.unwrap();

					let mod_info = MinecraftMod::try_parse(&tmp).await.unwrap();

					let old = self.mods.iter().find(|it| it.name == mod_info.name);
					let mod_dir = self.dir("mods").unwrap();
					if let Some(mcmod) = old {
						let old_file = mod_dir.join(mcmod.file_name.as_str());
						remove_file(&old_file).await.ok();
					}

					rename(tmp, mod_dir.join(filename.as_str())).await.ok();

					// don't restart if server is gracefully stopped
					if let Some(mc_server) = self._server_instance.as_ref() {
						let server_status = mc_server.status().await;

						if server_status != STOPPED {
							// no need for await
							self.restart_in_place();
						} else {
							// TODO: update config here
							//if  { }.	update_config(mc_server.create_config().await.unwrap()).await;
						}
					};
					return (StatusCode::OK, Json(Some(mod_info)));
				}
			}
		}
		(StatusCode::OK, Json(None))
	}

	#[inline]
	pub fn dir(&self, name: impl AsRef<Path>) -> Result<PathBuf> {
		self.config.dir(name)
	}

	pub async fn rm_file(&self, paths: Vec<String>) -> Result<()> {
		let mut removed: Vec<String> = Vec::with_capacity(paths.len());

		for file in paths {
			let target = self.dir(file.as_str());
			if let Ok(target) = target {
				if target.exists() {
					if metadata(&target).await?.is_dir() {
						if (remove_dir_all(target).await).is_ok() {
							removed.push(file);
						};
					} else if (remove_file(target).await).is_ok() {
						removed.push(file);
					}
				}
			}
		}
		Ok(())
	}


	pub async fn update_mc_file(&self, mut multipart: extract::Multipart) -> impl IntoResponse {
		while let Ok(Some(field)) = multipart.next_field().await {
			let name = field.name().unwrap().trim_start_matches(['/', '.']);
			let target = if let Ok(path) = self.dir(name) {
				path
			} else {
				return (StatusCode::FORBIDDEN, "");
			};
			let mut file = if target.exists() {
				File::create(target).await.unwrap()
			} else {
				create_dir_all(target.parent().unwrap()).await.ok();
				File::create(target).await.unwrap()
			};
			let mut reader = field;
			while let Some(Ok(data)) = reader.next().await {
				file.write_all(&data).await.ok();
			};
			file.flush().await.unwrap();
			file.shutdown().await.unwrap();
		}
		(StatusCode::OK, "Ok")
	}

	pub fn get_server(&self) -> Option<Arc<MinecraftServer>> {
		self._server_instance.as_ref().map(|it| Arc::clone(it))
	}

	pub fn restart_in_place(&mut self) -> JoinHandle<Result<()>> {
		match &self._server_instance {
			None => { unreachable!() }
			Some(server) => {
				let cfg = Arc::clone(&self.config);
				let server = Arc::clone(server);
				spawn(async move {
					server.restart_in_place(move || {
						Box::pin(async move {
							cfg.spawn().await
						})
					}).await
				})
			}
		}
	}

	pub async fn scan_mods(&self) -> Result<Vec<MinecraftMod>> {
		let mod_dir = self.dir("mods")?;
		if !mod_dir.exists() {
			return Ok(Vec::new());
		}
		let files = crate::file_scanner::scan_files(mod_dir, |it| it.file_name().to_string_lossy().ends_with(".jar")).await?;
		let file_infos = FuturesUnordered::new();
		for x in files {
			file_infos.push(MinecraftMod::try_parse(x));
		}

		let f = file_infos.collect::<Vec<_>>().await.into_iter()
			.filter(|it| it.is_ok())
			.map(|it| it.unwrap()).collect();
		Ok(f)
	}

	async fn download_server(&self) -> io::Result<()> {
		self.mod_type.download_server(&new_client()?,
		                              &self.version,
		                              &self.config.directory).await
	}
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ModType {
	Vanilla,
	Purpur,
	Forge(String),
}

impl Default for ModType {
	fn default() -> Self { Self::Vanilla }
}

impl ModType {
	pub async fn download_server(&self, client: &Client, mc_version: &str, target: impl AsRef<Path>) -> io::Result<()> {
		let url = match self {
			ModType::Vanilla => {
				return Err(io::Error::new(ErrorKind::Unsupported, anyhow!("Unsupported")));
			}
			ModType::Purpur => {
				format!("https://api.purpurmc.org/v2/purpur/{mc_version}/latest/download")
			}
			ModType::Forge(ver) => {
				format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{mc_version}-{ver}/forge-{mc_version}-{ver}-installer.jar")
			}
		};
		download_to(client, &url, target).await?;
		Ok(())
	}

	pub async fn versions(&self, client: &Client) -> io::Result<ModVersion> {
		let mut res = ModVersion { versions: Default::default() };
		match self {
			ModType::Vanilla => {
				return Err(io::Error::new(ErrorKind::Unsupported, anyhow!("Unsupported")));
			}
			ModType::Purpur => {
				let resp = client.get("https://api.purpurmc.org/v2/purpur")
					.send().await.map_err(reqwest_to_io)?;
				let versions: PurpurVersions = resp.json().await.map_err(reqwest_to_io)?;
				let mut table = HashMap::new();
				for ver in versions.versions {
					table.insert(ver, ModVersionInfo { recommended: None, latest: String::new() });
				}
				res.versions = table;
			}
			ModType::Forge(_) => {
				let resp = client.get("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json")
					.send().await.map_err(reqwest_to_io)?;
				let versions: ForgeVersions = resp.json().await.map_err(reqwest_to_io)?;
				let mut table = HashMap::new();
				for (k, v) in versions.promos {
					let mut ver_info = k.split('-');
					let mc_ver = ver_info.next();
					let ver_type = ver_info.next();
					if let (Some(ver), Some(typ)) = (mc_ver, ver_type) {
						let ent = table.entry(ver.to_string())
							.or_insert(ModVersionInfo {
								recommended: None,
								latest: String::new(),
							});
						match typ {
							"recommended" => {
								ent.recommended = Some(v);
							}
							_ => {
								ent.latest = v;
							}
						}
					}
				}
				res.versions = table;
			}
		}
		Ok(res)
	}
}

#[derive(Deserialize)]
struct PurpurVersions {
	project: String,
	versions: Vec<String>,
}

#[derive(Deserialize)]
struct ForgeVersions {
	homepage: String,
	promos: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModVersionInfo {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	recommended: Option<String>,
	latest: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModVersion {
	versions: HashMap<String, ModVersionInfo>,
}

impl ModVersion {
	pub fn latest(&self) -> Option<&str> {
		self.versions.keys().max_by(|a, b| cmp_semver(a, b)).map(|it| it.as_str())
	}

	pub fn latest_for(&self, ver: &str) -> Option<&str> {
		self.versions.get(ver).map(|it| it.latest.as_str())
	}
}

fn cmp_semver(left: &str, right: &str) -> Ordering {
	let left = left.split('.').map(|it| it.parse::<u64>().ok()).collect::<Vec<Option<u64>>>();
	let right = right.split('.').map(|it| it.parse::<u64>().ok()).collect::<Vec<Option<u64>>>();
	let n = left.len().min(right.len());
	for i in 0..n {
		match (left[i], right[i]) {
			(None, None) => {
				return Ordering::Equal;
			}
			(None, Some(_)) => {
				return Ordering::Less;
			}
			(Some(_), None) => {
				return Ordering::Greater;
			}
			(Some(a), Some(b)) => {
				let ord = a.cmp(&b);
				if ord != Ordering::Equal {
					return ord;
				}
			}
		}
	}
	left.len().cmp(&right.len())
}