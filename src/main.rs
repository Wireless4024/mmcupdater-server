#![feature(str_split_as_str)]

extern crate core;

use std::io::{Bytes, Write};
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use axum::{async_trait, extract, Json, Router};
use axum::body::{Body, BoxBody, boxed};
use axum::extract::{FromRequest, RequestParts};
use axum::http::{HeaderMap, Request, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use base32::Alphabet;
use bytes::Buf;
use futures::{StreamExt, TryStreamExt};
use futures::future::ok;
use rand::prelude::*;
use tokio::fs::{create_dir_all, File, OpenOptions, remove_file, rename};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{OnceCell, RwLock};
use tower::ServiceExt;
use tower_http::services::ServeDir;
use tracing::info;

use crate::config::{Config, Minecraft, MinecraftServer};
use crate::config::MinecraftServerStatus::RUNNING;
use crate::jar_scanner::get_manifest;
use crate::minecraft_mod::MinecraftMod;
use crate::schema::MinecraftServerConfig;

mod config;
mod file_scanner;
mod file_info;
mod schema;
mod jar_scanner;
mod minecraft_mod;

static SERVER_CONFIG: OnceCell<RwLock<MinecraftServerConfig>> = OnceCell::const_new();
static MCSERVER: OnceCell<RwLock<MinecraftServer>> = OnceCell::const_new();

async fn get_server() -> &'static RwLock<MinecraftServerConfig> {
	SERVER_CONFIG.get().unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
	dotenv::dotenv().ok();
//	println!("{:?}", get_manifest("mc/mods/Quark-r2.4-321.jar").await?);;
	if std::env::var_os("RUST_LOG").is_none() {
		std::env::set_var("RUST_LOG", "debug")
	}
	if std::env::var_os("auth").is_none() {
		let mut secret = [0u8; 64];
		let mut rng = rand::thread_rng();
		rng.try_fill_bytes(&mut secret);
		let mut secret = hex::encode(secret);
		secret.reserve_exact(6);
		secret.insert_str(0, "auth=");
		secret.push('\n');
		std::env::set_var("auth", secret.as_str());
		let mut file = OpenOptions::new().create(true).write(true).append(true).open(".env").await?;
		file.write_all(secret.as_bytes()).await?;
		file.shutdown().await?;
	}
	tracing_subscriber::fmt::init();
	let app = Router::new()
		.route("/config.json", get(config))
		.route("/stop", get(shutdown))
		.route("/status", get(status))
		.route("/restart", get(restart))
		.route("/update", post(update))
		.nest("/mods", get(handler));

	tokio::spawn(async move {
		let mut default_config_file = File::open("default_server.json").await?;
		let mut data = String::new();
		default_config_file.read_to_string(&mut data).await?;

		let cfg: Config = serde_json::from_str(data.as_str())?;

		let config = cfg.minecraft.create_config().await?;

		SERVER_CONFIG.set(RwLock::new(config)).ok();

		let mut server = cfg.minecraft.build()?;
		MCSERVER.set(RwLock::new(server)).ok();

		Result::<()>::Ok(())
	});

	let defaut_addr = SocketAddr::from(([0, 0, 0, 0], 8888));

	let addr = if let Ok(listen) = std::env::var("http_listen") {
		listen.parse().unwrap_or(defaut_addr)
	} else {
		defaut_addr
	};
	axum::Server::bind(&addr)
		.serve(app.into_make_service())
		.await
		.unwrap();
	Ok(())
}

async fn config() -> impl IntoResponse {
	(StatusCode::OK, Json((*get_server().await.read().await).clone()))
}

async fn handler(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
	let res = get_static_file(uri.clone()).await?;
	Ok(res)
}

async fn get_static_file(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
	let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
	// `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
	match ServeDir::new("mc/mods").oneshot(req).await {
		Ok(res) => Ok(res.map(boxed)),
		Err(err) => Err((
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("Something went wrong: {}", err),
		)),
	}
}

async fn shutdown(_: Protected) -> impl IntoResponse {
	let mut server = MCSERVER.get().unwrap().read().await;
	server.shutdown_in_place().await.ok();
	std::process::exit(0);
	""
}

async fn restart_server() -> Result<()> {
	let mut server = MCSERVER.get().unwrap().read().await;
	server.update_config(SERVER_CONFIG.get().unwrap().write().await).await.ok();
	server.restart_in_place().await.unwrap();
	Ok(())
}

async fn restart(_: Protected) -> impl IntoResponse {
	restart_server().await.ok();
	"Ok"
}

async fn status() -> impl IntoResponse {
	let server = MCSERVER.get().unwrap().read().await;
	format!("{:?}", server.status().await)
}

async fn update(mut multipart: extract::Multipart, _: Protected) -> impl IntoResponse {
	while let Ok(Some(mut field)) = multipart.next_field().await {
		let name = field.name().unwrap();
		if let "file" = name {
			let filename = field.file_name().and_then(|it| it.split("/").last()).map(|it| it.to_string());
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

				let server = MCSERVER.get().unwrap().write().await;
				let mc_server = server;

				let mod_info = MinecraftMod::new(&tmp).await.unwrap();

				let sconfig = SERVER_CONFIG.get().unwrap().read().await;
				let old = sconfig.mods.iter().find(|it| it.name == mod_info.name);
				let mod_dir = mc_server.dir("mods");
				if let Some(mcmod) = old {
					let old_file = mod_dir.join(mcmod.file_name.as_str());
					remove_file(&old_file).await.ok();
				}

				rename(tmp, mod_dir.join(filename.as_str())).await.ok();

				tokio::spawn(async {
					restart_server().await.ok();
				});
				break;
			}
		}
	}
	"Ok"
}

pub(crate) struct Protected;

#[async_trait]
impl<B: Send> FromRequest<B> for Protected {
	type Rejection = AuthError;

	async fn from_request(req: &mut RequestParts<B>) -> std::result::Result<Self, Self::Rejection> {
		if let Some(headers) = req.headers() {
			if let Some(key) = headers.get("Authorization") {
				if key.to_str().unwrap() == std::env::var("auth").unwrap().as_str() {
					return Ok(Protected);
				}
			}
		};
		Err(AuthError)
	}
}

struct AuthError;

impl IntoResponse for AuthError {
	fn into_response(self) -> axum::response::Response {
		(StatusCode::UNAUTHORIZED, "").into_response()
	}
}