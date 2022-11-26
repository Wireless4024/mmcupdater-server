use std::{io, mem};
use std::convert::AsRef;
use std::ffi::OsString;
use std::fs::Permissions;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use bstr::ByteSlice;
use pedestal_rs::fs::path::relative_from;
use reqwest::Client;
use tokio::fs;
use tokio::fs::{create_dir_all, metadata, read_dir, remove_dir_all, remove_file, rename};
use tokio::sync::RwLock;
use tracing::{debug, error, warn};
use tracing::log::info;

use crate::util::errors::{ErrorWrapper, Result};
use crate::util::fs::extract_archive;
use crate::util::gh::{get_gh_latest_release, get_gh_release_from_tag, GhRelease, ReleaseAsset};
use crate::util::http::{download_to, new_client};
use crate::util::platform::{ARCH, OS};
use crate::util::process::eval;

pub struct Java {
	java_path: String,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone)]
pub struct JavaInfo {
	id: String,
	path: String,
	version: u8,
	is_graalvm: Option<bool>,
}

impl Java {
	pub fn new(path: impl Into<String>) -> Java {
		Self {
			java_path: path.into()
		}
	}


	pub async fn get_info(&self) -> io::Result<JavaInfo> {
		let out = eval([&self.java_path, "-version"]).await?;
		let version = (|| {
			let mut lines = out.split(|b| b == &b'\n');
			let info = lines.next()?;
			let mut tokens = info.split(|b| b == &b'"');
			tokens.next()?;// java version "
			let ver = tokens.next()?; // x.y.z
			let mut vers = ver.split(|it| it == &b'.');
			let first = vers.next()?;
			if first != b"1" {
				String::from_utf8_lossy(first).parse::<u8>().ok()
			} else {
				// java version < 9 use 1. prefix eg. 1.8 for java 8
				String::from_utf8_lossy(vers.next()?).parse::<u8>().ok()
			}
		})();
		let is_graalvm = if out.contains_str("GraalVM") {
			Some(out.contains_str("EE"))
		} else {
			None
		};
		let version = version.ok_or_else(|| io::Error::from(ErrorKind::InvalidFilename))?;
		let mut id = String::new();
		id.push_str("java-");
		id.push_str(&version.to_string());
		if let Some(is_graalvm) = is_graalvm {
			id.push_str("-graalvm");
			if is_graalvm {
				id.push_str("-ee");
			}
		}
		Ok(JavaInfo {
			id,
			version,
			path: self.java_path.to_string(),
			is_graalvm,
		})
	}
}

impl JavaInfo {
	pub fn performance_args(&self) -> &'static [&'static str] {
		// graalvm provide faster cold startup time than other java provider
		match self.is_graalvm {
			None => {
				&[]
			}
			Some(false) => {
				&["-XX:+UseJVMCICompiler", "-XX:+UseJVMCINativeLibrary"]
			}
			Some(true) => {
				&[
					"-XX:+UseJVMCICompiler",
					"-XX:+UseJVMCINativeLibrary",
					"-Dgraal.UsePriorityInlining=true",
					"-Dgraal.TuneInlinerExploration=1",
					"-Dgraal.CompilerConfiguration=enterprise",
					"-Dgraal.MitigateSpeculativeExecutionAttacks=none"
				]
			}
		}
	}

	pub fn path_for(&self, dir: impl AsRef<Path>) -> OsString {
		if self.path.contains('/') {
			relative_from(&self.path, dir).into_os_string()
		} else {
			// maybe it's executable that need to look up from $PATH just return it directly
			OsString::from(&self.path)
		}
	}
}

static JAVA_RUNTIME_DIR: &str = "java_runtime";

pub static JAVA_RUNTIMES: RwLock<Vec<JavaInfo>> = RwLock::const_new(Vec::new());

pub struct JavaManager;

impl JavaManager {
	pub async fn versions() -> Vec<JavaInfo> {
		let runtimes = JAVA_RUNTIMES.read().await;
		runtimes.clone()
	}

	pub async fn get_by_id(id: &str) -> Option<JavaInfo> {
		let runtimes = JAVA_RUNTIMES.read().await;
		runtimes.iter().find(|it| it.id == id).cloned()
	}

	pub async fn get_version(version: u8) -> Option<JavaInfo> {
		let runtimes = JAVA_RUNTIMES.read().await;
		let existing = runtimes.iter().find(|it| it.version == version);
		if existing.is_none() {
			info!("java version {version} was not found");
			match version {
				8 | 11 | 17 => {
					info!("downloading java version {version}..");
					return Self::download_version(version).await.ok();
				}
				_ => {}
			}
		}
		existing.cloned()
	}

	pub async fn scan() -> io::Result<()> {
		let mut javas = vec![];
		info!("scanning for java in {JAVA_RUNTIME_DIR:?}");
		if metadata(JAVA_RUNTIME_DIR).await.is_ok() {
			let mut folders = read_dir(JAVA_RUNTIME_DIR).await?;
			while let Some(folder) = folders.next_entry().await? {
				if folder.file_type().await?.is_dir() {
					let mut path = folder.path();
					path.push("bin/java");
					let java = Java::new(path.to_string_lossy());
					let info = java.get_info().await?;
					debug!("found {path:?} version {}",info.version);
					javas.push(info);
				}
			}
		}
		// scan regular java in $PATH
		if let Ok(info) = Java::new("java").get_info().await {
			info!("scanning for java in $PATH");
			debug!("found \"{}\" version {}", info.path, info.version);
			javas.push(info);
		}
		*JAVA_RUNTIMES.write().await = javas;
		Ok(())
	}

	pub async fn download_version(ver: u8) -> Result<JavaInfo> {
		if metadata(JAVA_RUNTIME_DIR).await.is_err() {
			create_dir_all(JAVA_RUNTIME_DIR).await?;
		}
		match ver {
			8 => {
				debug!("preparing to download java 8");
				let release = get_gh_release_from_tag(GRAALVM_REPO, GRAALVM_JAVA8_LATEST_KNOWN_TAG).await?;
				let release = GraalVmRelease::new(release.ok_or(ErrorWrapper::NotFound)?);
				// wont fail maybe
				let java = Self::download_java_impl(&new_client()?, ver, &release).await?.expect("Find java 8 release from graalvm");
				return Ok(java);
			}
			// TODO: use macro
			11 => {
				debug!("preparing to download java 11");
				let release = get_gh_latest_release(GRAALVM_REPO).await?
					.and_then(|it| {
						let release = GraalVmRelease::new(it);
						if release.find_capable_release(11).is_some() {
							Some(release)
						} else {
							None
						}
					});
				let release = match release {
					Some(it) => { it }
					None => {
						let release = get_gh_release_from_tag(GRAALVM_REPO, GRAALVM_JAVA11_LATEST_KNOWN_TAG).await?.ok_or(ErrorWrapper::NotFound)?;
						let release = GraalVmRelease::new(release);
						if release.find_capable_release(11).is_some() {
							release
						} else {
							return Err(ErrorWrapper::NotFound);
						}
					}
				};
				let java = Self::download_java_impl(&new_client()?, 11, &release).await?.expect("Find java 11 release from graalvm");
				return Ok(java);
			}
			17 => {
				debug!("preparing to download java 17");
				let release = get_gh_latest_release(GRAALVM_REPO).await?
					.and_then(|it| {
						let release = GraalVmRelease::new(it);
						if release.find_capable_release(17).is_some() {
							Some(release)
						} else {
							None
						}
					});
				let release = match release {
					Some(it) => { it }
					None => {
						let release = get_gh_release_from_tag(GRAALVM_REPO, GRAALVM_JAVA17_LATEST_KNOWN_TAG).await?.ok_or(ErrorWrapper::NotFound)?;
						let release = GraalVmRelease::new(release);
						if release.find_capable_release(17).is_some() {
							release
						} else {
							return Err(ErrorWrapper::NotFound);
						}
					}
				};
				let java = Self::download_java_impl(&new_client()?, 17, &release).await?.expect("Find java 17 release from graalvm");
				return Ok(java);
			}
			_ => {
				Err(io::Error::new(ErrorKind::Unsupported, format!("java version {ver} is unsupported")))?;
			}
		}
		unreachable!()
	}

	async fn download_java_impl(client: &Client, version: u8, release: &GraalVmRelease) -> Result<Option<JavaInfo>> {
		let (zip, hash_asset) = match release.find_capable_release(version) {
			Some(it) => { it }
			None => {
				return Ok(None);
			}
		};
		let zip = Self::download_file(client,
		                              &zip.browser_download_url,
		                              hash_asset.map(|it| it.browser_download_url.as_str()).unwrap_or("")).await?;
		let outdir = AsRef::<Path>::as_ref(JAVA_RUNTIME_DIR).join(format!("java{version}"));
		info!("extracting file {zip:?} to {outdir:?}");
		extract_archive(zip, &outdir).await?;
		let java = Self::try_purge_jdk(&outdir).await?;

		JAVA_RUNTIMES.write().await.push(java.clone());
		info!("java {version} has been downloaded");
		Ok(Some(java))
	}

	async fn download_file(client: &Client, file_url: &str, hash_url: &str) -> Result<PathBuf> {
		debug!("Downloading java archive from {file_url}");
		let (out_file, hash) = download_to(client, file_url, JAVA_RUNTIME_DIR).await?;

		if hash_url.is_empty() {
			warn!("Downloaded file at '{out_file:?}' but can't verify its hash");
			return Ok(out_file);
		}
		debug!("Verifying hash from {file_url}");
		let expected_hash = client.get(hash_url).send().await?.text().await?;
		if hash != expected_hash {
			error!("Fail to verify hash from downloaded file path={out_file:?}");
			Err(io::Error::new(ErrorKind::InvalidData, "Invalid hash"))?;
		}
		Ok(out_file)
	}

	pub async fn try_purge_jdk(folder: impl AsRef<Path>) -> io::Result<JavaInfo> {
		let folder = folder.as_ref();
		// have bundled jre
		let jre = folder.join("jre");
		if metadata(&jre).await.is_ok() {
			let parent = folder.parent().unwrap();
			// may cause data race
			let temp = parent.join("tmp");
			rename(jre, &temp).await?;
			remove_dir_all(folder).await?;
			rename(temp, folder).await?;
		} else {
			remove_file(folder.join("src.zip")).await.ok();
		}

		let java_path = folder.join("bin/java");
		#[cfg(target_os = "linux")]
		{
			use std::os::unix::prelude::PermissionsExt;
			let mut perm = metadata(&java_path).await?.permissions();
			Permissions::set_mode(&mut perm, 0o755);
			fs::set_permissions(&java_path, perm).await?;
		}
		Java::new(java_path.to_string_lossy()).get_info().await
	}
}

pub async fn get_graalvm_release() -> Result<GraalVmRelease> {
	let release = get_gh_latest_release(GRAALVM_REPO).await?.ok_or(ErrorWrapper::NotFound)?;
	Ok(GraalVmRelease::new(release))
}

#[cfg_attr(debug_assertions, derive(Debug))]
pub struct GraalVmRelease {
	inner: GhRelease,
}

static GRAALVM_REPO: &str = "graalvm/graalvm-ce-builds";
static GRAALVM_RELEASE_FILTER_NAME: &str = "GraalVM Community Edition";

/// this is fallback because graalvm already dropped support for java 8
static GRAALVM_JAVA8_LATEST_KNOWN_TAG: &str = "vm-21.3.1";

/// this is fallback in future if graalvm drop support on java 11
static GRAALVM_JAVA11_LATEST_KNOWN_TAG: &str = "vm-22.3.0";

/// this is fallback in future if graalvm drop support on java 17 (should be fine until September 2029)
static GRAALVM_JAVA17_LATEST_KNOWN_TAG: &str = GRAALVM_JAVA11_LATEST_KNOWN_TAG;

impl GraalVmRelease {
	pub fn new(mut release: GhRelease) -> Self {
		let assets = mem::take(&mut release.assets);
		release.assets = assets
			.into_iter()
			.filter(|it| it.name.ends_with(".tar.gz") || it.name.ends_with(".zip"))
			.collect();

		Self {
			inner: release
		}
	}

	pub fn find_capable_release(&self, java_version: u8) -> Option<(&ReleaseAsset, Option<&ReleaseAsset>)> {
		let mut asset = self.inner
			.assets
			.iter()
			.filter(|it| it.name.starts_with(&format!("graalvm-ce-java{java_version}")))
			.filter(|it| it.name.contains(OS) && it.name.contains(ARCH));
		let mut asset = (asset.next()?, asset.next());
		if asset.0.name.ends_with("sha256") {
			asset = (asset.1?, Some(asset.0));
		}
		Some(asset)
	}
}