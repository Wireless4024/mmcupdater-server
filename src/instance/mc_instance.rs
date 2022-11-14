use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::{extract, Json};
use axum::body::{Body, BoxBody, boxed};
use axum::http::{Request, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use futures::StreamExt;
use pedestal_rs::fs::path::normalize;
use serde::{Deserialize, Serialize};
use tokio::fs::{create_dir_all, File, remove_dir_all, remove_file, rename};
use tokio::io::AsyncWriteExt;
use tokio::task::{JoinHandle, spawn_local};
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::config::MinecraftConfig;
use crate::instance::mc_mod::MinecraftMod;
use crate::instance::mc_server::MinecraftServer;
use crate::instance::mc_server::MinecraftServerStatus::STOPPED;

#[derive(Serialize, Deserialize)]
pub struct McInstance {
	name: String,
	version: String,
	folder: String,
	config: Arc<MinecraftConfig>,
	#[serde(default)]
	mod_type: ModType,
	#[serde(default)]
	mods: Vec<MinecraftMod>,
	#[serde(skip)]
	_server_instance: Option<Arc<MinecraftServer>>,
}

impl McInstance {
	async fn get_web_file(&self, uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
		let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
		match ServeDir::new(&self.folder).oneshot(req).await {
			Ok(res) => Ok(res.map(boxed)),
			Err(err) => Err(
				(
					StatusCode::INTERNAL_SERVER_ERROR,
					format!("Something went wrong: {}", err)
				)
			),
		}
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

	pub fn dir(&self, name: impl AsRef<Path>) -> io::Result<PathBuf> {
		normalize(self.folder.as_ref(), name)
	}

	pub async fn rm_file(&self, paths: Vec<String>) -> io::Result<()> {
		let mut removed: Vec<String> = Vec::with_capacity(paths.len());

		for file in paths {
			let target = self.dir(file.as_str());
			if let Ok(target) = target {
				if target.exists() {
					if target.is_dir() {
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

	pub fn restart_in_place(&mut self) -> JoinHandle<io::Result<()>> {
		match &self._server_instance {
			None => {
				tokio::spawn(async { Ok(()) })
			}
			Some(server) => {
				let cfg = Arc::clone(&self.config);
				let server = Arc::clone(server);
				spawn_local(async move {
					server.restart_in_place(move || {
						Box::pin(async move {
							cfg.spawn().await
						})
					}).await
				})
			}
		}
	}
}

#[derive(Serialize, Deserialize)]
pub enum ModType {
	Vanilla,
	Forge(String),
}

impl Default for ModType {
	fn default() -> Self { Self::Vanilla }
}