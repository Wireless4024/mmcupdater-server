use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Extension;
use hashbrown::HashMap;
use pedestal_rs::fs::path::normalize;
use tokio::fs::{create_dir_all, File, read_dir};
use tokio::io;
use tokio::sync::{OwnedRwLockWriteGuard, RwLock};
use tracing::{debug, error, info};

use crate::instance::mc_instance::{McInstance, ModType};

type Instance = Arc<RwLock<McInstance>>;
pub type InstanceManagerExt = Extension<Arc<RwLock<InstanceManager>>>;

pub struct InstanceManager {
	pub instances: HashMap<String, Instance>,
	folder: String,
}


macro_rules! instance_async {
    ($self:expr, $name:ident, $var:ident, $block:expr) => {
	    $self.with_instance_async($name.as_ref(), |mut $var| {
			async move { $block }
		}).await
    };
}

impl InstanceManager {
	pub fn new() -> Self {
		Self {
			instances: HashMap::new(),
			folder: String::from("instances"),
		}
	}

	pub fn into_extension(self) -> InstanceManagerExt {
		Extension(Arc::new(RwLock::new(self)))
	}

	pub async fn init(&mut self) -> io::Result<()> {
		let path: &Path = self.folder.as_ref();
		if !path.exists() {
			info!("creating new instance folder at {path:?}");
			create_dir_all(path).await?;
		}
		self.update().await?;
		Ok(())
	}

	pub async fn update(&mut self) -> io::Result<()> {
		info!("scanning {:?} for minecraft instances", self.folder);
		let mut dir = read_dir(&self.folder).await?;
		while let Ok(Some(e)) = dir.next_entry().await {
			if let Ok(typ) = e.file_type().await {
				if typ.is_dir() {
					match McInstance::load(e.path()).await {
						Ok(mut instance) => {
							let name = instance.name.clone();
							match instance.init().await {
								Ok(_) => {
									debug!("found {name:?} at \"instances{}\"", instance.config.directory.rsplit(&self.folder).next().unwrap());
									self.instances.insert(name, Arc::new(RwLock::new(instance)));
								}
								Err(err) => {
									error!("Error while loading instance {name} cause by {err:#?}")
								}
							}
						}
						Err(err) => {
							error!("Failed to load instance at `{:?}` due `{}`", e.path(), err);
						}
					}
				}
			}
		}
		Ok(())
	}

	pub async fn new_instance(&mut self, name: &str, version: &str, typ: ModType) -> io::Result<Instance> {
		let path = normalize(self.folder.as_ref(), name)?;
		if !path.exists() {
			create_dir_all(&path).await?;
		}
		let instance = McInstance::generate(&path, version, typ).await?;
		let name = instance.name.clone();
		let instance = Arc::new(RwLock::new(instance));
		self.instances.insert(name, instance.clone());
		Ok(instance)
	}

	pub fn names(&self) -> Vec<&str> {
		self.instances.keys().map(|it| it.as_str()).collect()
	}

	pub fn find(&self, name: &str) -> Option<Instance> {
		self.instances.get(name).map(Arc::clone)
	}

	pub async fn with_instance_async<'a, T, Fut: Future<Output=T> + 'a, Fn: FnOnce(OwnedRwLockWriteGuard<McInstance>) -> Fut>(&'a self, name: &str, block: Fn) -> Option<T> {
		let _instance = self.find(name)?;
		let instance = _instance.write_owned().await;
		Some(block(instance).await)
	}

	/// return bool: true if instance is found
	pub async fn restart(&self, name: impl AsRef<str>) -> bool {
		instance_async!(self, name, instance, {
			instance.restart_in_place().await.ok();
		}).is_some()
	}

	/// return bool: true if instance is found
	pub async fn stop(&self, name: impl AsRef<str>) -> bool {
		instance_async!(self, name, instance, {
			if let Some(server) = instance.get_server() {
				server.shutdown_in_place().await.ok();
			}
		}).is_some()
	}

	/// return bool: true if instance is found
	pub async fn kill(&self, name: impl AsRef<str>) -> bool {
		instance_async!(self, name, instance, {
			if let Some(server) = instance.get_server() {
				server.kill().await.ok();
			}
		}).is_some()
	}

	/// return bool: true if instance is found
	pub async fn input(&self, name: impl AsRef<str>, message: Vec<u8>) -> bool {
		instance_async!(self, name, instance, {
			if let Some(server) = instance.get_server() {
				server.input(&message).await.ok();
			}
		}).is_some()
	}

	/// return bool: true if instance is found
	pub async fn say(&self, name: impl AsRef<str>, message: String) -> bool {
		instance_async!(self, name, instance, {
			if let Some(server) = instance.get_server() {
				server.say(message).await.ok();
			}
		}).is_some()
	}

	/// return bool: Option<File> if instance is found and file is valid
	/// Response using https://github.com/tokio-rs/axum/discussions/608#discussioncomment-1789020
	pub async fn get_file(&self, name: impl AsRef<str>, path: impl AsRef<str>) -> Option<File> {
		let path = PathBuf::from(path.as_ref());
		let res = instance_async!(self, name, instance, {
			instance.get_file(path).await
		});
		res.flatten()
	}
}