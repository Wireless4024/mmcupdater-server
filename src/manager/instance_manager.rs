use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use axum::Extension;
use dashmap::DashMap;
use pedestal_rs::fs::path::normalize;
use tokio::fs::{create_dir_all, File, read_dir, remove_dir};
use tokio::sync::{OwnedRwLockWriteGuard, RwLock};
use tracing::{debug, error, info};

use crate::instance::mc_instance::{McInstance, ModType};

type Instance = Arc<RwLock<McInstance>>;
pub type InstanceManagerExt = Extension<Arc<RwLock<InstanceManager>>>;

pub struct InstanceManager {
	pub instances: DashMap<String, Instance>,
	folder: String,
}

macro_rules! instance_async {
    ($self:expr, $name:ident, $var:ident, $block:expr) => {
	    match $self.find($name.as_ref()) {
		    Some(it) => {
			    let mut $var = it.write().await;
			    Some($block)
		    }
		    None => {
			    None
		    }
	    }
    };
}

///  same as `instance_async` but read only
macro_rules! instance_async_ro {
    ($self:expr, $name:ident, $var:ident, $block:expr) => {
	    match $self.find($name.as_ref()) {
		    Some(it) => {
			    let $var = it.read().await;
			    Some($block)
		    }
		    None => {
			    None
		    }
	    }
    };
}

impl InstanceManager {
	pub fn new() -> Self {
		Self {
			instances: Default::default(),
			folder: String::from("instances"),
		}
	}

	pub fn into_extension(self) -> InstanceManagerExt {
		Extension(Arc::new(RwLock::new(self)))
	}

	pub async fn init(&mut self) -> Result<()> {
		let path: &Path = self.folder.as_ref();
		if !path.exists() {
			info!("creating new instance folder at {path:?}");
			create_dir_all(path).await?;
		}
		self.update().await?;
		Ok(())
	}

	pub async fn update(&mut self) -> Result<()> {
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

	pub async fn new_instance(&self, name: &str, version: &str, typ: ModType) -> Result<Instance> {
		let path = normalize(self.folder.as_ref(), name)?;
		if !path.exists() {
			create_dir_all(&path).await?;
		}
		let instance = McInstance::generate(&path, version, typ).await?;
		let name = instance.name.clone();
		let instance = Arc::new(RwLock::new(instance));
		self.instances.insert(name, Arc::clone(&instance));
		Ok(instance)
	}

	pub async fn remove_instance(&self, name: &str) -> Result<Option<Instance>> {
		let path = normalize(self.folder.as_ref(), name)?;
		if let Some((_, instance)) = self.instances.remove(name) {
			if path.exists() {
				remove_dir(instance.read().await.dir("").unwrap()).await?;
			}
			Ok(Some(instance))
		} else {
			Ok(None)
		}
	}

	pub fn names(&self) -> Vec<String> {
		self.instances.iter().map(|it| it.key().as_str().to_string()).collect()
	}

	pub fn find(&self, name: &str) -> Option<Instance> {
		self.instances.get(name).map(|it| Arc::clone(it.value()))
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
		instance_async_ro!(self, name, instance, {
			if let Some(server) = instance.get_server() {
				server.shutdown_in_place().await.ok();
			}
		}).is_some()
	}

	/// return bool: true if instance is found
	pub async fn kill(&self, name: impl AsRef<str>) -> bool {
		instance_async_ro!(self, name, instance, {
			if let Some(server) = instance.get_server() {
				server.kill().await.ok();
			}
		}).is_some()
	}

	/// return bool: true if instance is found
	pub async fn input(&self, name: impl AsRef<str>, message: Vec<u8>) -> bool {
		instance_async_ro!(self, name, instance, {
			if let Some(server) = instance.get_server() {
				server.input(&message).await.ok();
			}
		}).is_some()
	}

	/// return bool: true if instance is found
	pub async fn say(&self, name: impl AsRef<str>, message: String) -> bool {
		instance_async_ro!(self, name, instance, {
			if let Some(server) = instance.get_server() {
				server.say(message).await.ok();
			}
		}).is_some()
	}

	/// return bool: Option<File> if instance is found and file is valid
	/// Response using https://github.com/tokio-rs/axum/discussions/608#discussioncomment-1789020
	pub async fn get_file(&self, name: impl AsRef<str>, path: impl AsRef<str>) -> Option<File> {
		let path = PathBuf::from(path.as_ref());
		let res = instance_async_ro!(self, name, instance, {
			instance.get_file(path).await
		});
		res.flatten()
	}
}