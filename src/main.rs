#![feature(str_split_as_str)]
#![forbid(unsafe_code)]

extern crate core;

use std::env;
use std::fmt::format;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;

use anyhow::Result;
use axum::{async_trait, extract, Json, Router};
use axum::body::{Body, BoxBody, boxed};
use axum::extract::{FromRequest, Query, RequestParts};
use axum::http::{HeaderMap, HeaderValue, Request, Response, StatusCode, Uri};
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use futures::StreamExt;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::fs::{create_dir_all, File, OpenOptions, remove_dir_all, remove_file, rename};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{OnceCell, RwLock};
use tower::util::ServiceExt;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{debug, info, warn};

use crate::config::{Config, MinecraftServer};
use crate::config::MinecraftServerStatus::{RUNNING, STOPPED};
use crate::jar_scanner::get_manifest;
use crate::minecraft_mod::MinecraftMod;
use crate::schema::{ForgeInfo, MinecraftServerConfig};
use crate::util::get_zip_file;

mod config;
mod file_scanner;
mod file_info;
mod schema;
mod jar_scanner;
mod minecraft_mod;
mod util;

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
		std::env::set_var("RUST_LOG", "info")
	}
	if std::env::var_os("auth").is_none() {
		let mut secret = [0u8; 64];
		let mut rng = rand::thread_rng();
		rng.try_fill_bytes(&mut secret)?;
		let mut secret = hex::encode(secret);
		std::env::set_var("auth", secret.as_str());
		secret.reserve_exact(6);
		secret.insert_str(0, "auth=");
		secret.push('\n');
		let mut file = OpenOptions::new().create(true).write(true).append(true).open(".env").await?;
		file.write_all(secret.as_bytes()).await?;
		file.shutdown().await?;
	}
	tracing_subscriber::fmt::init();
	let app = Router::new()
		.route("/stop", get(shutdown))
		.route("/kill", get(kill))
		.route("/status", get(status))
		.route("/restart", get(restart))
		.route("/update", post(update))
		.route("/update_cfg", post(update_cfg))
		.nest("/mc/file", get(get_mc_file))
		.route("/mc/file", post(list_mc_file))
		.route("/mc/file", put(update_mc_file))
		.route("/mc/file", delete(rm_mc_file))
		.layer(CorsLayer::permissive());
	let mut app = app
		.nest("/mods", get(handler))
		.route("/config.json", get(config));

	if let Ok(mut serve) = env::var("serve_config") {
		serve.make_ascii_lowercase();
		match serve.as_str() {
			"y" | "1" | "true" => {
				app = app.route("/config.zip", get(config_dir));
			}
			_ => {}
		}
	}

	if Path::new("web").exists() {
		info!("found web folder adding route to it");
		app = app.nest("/web", get(get_web_file))
	}
	tokio::spawn(async move {
		let mut default_config_file = File::open("default_server.json").await?;
		let mut data = String::new();
		default_config_file.read_to_string(&mut data).await?;
		info!("Loading default server");

		let cfg: Config = serde_json::from_str(data.as_str()).unwrap();

		let config = cfg.minecraft.create_config().await.unwrap();

		SERVER_CONFIG.set(RwLock::new(config)).ok();
		info!("Building minecraft instance");
		let server = cfg.minecraft.build().unwrap();
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
		.tcp_nodelay(true)
		.serve(app.into_make_service())
		.await
		.unwrap();
	Ok(())
}

async fn config() -> impl IntoResponse {
	(StatusCode::OK, Json((*get_server().await.read().await).clone()))
}

async fn config_dir() -> impl IntoResponse {
	let server = MCSERVER.get().unwrap().read().await;

	let data = server.get_config_zip().await.unwrap();
	let mut headers = HeaderMap::new();
	headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/zip"));
	(StatusCode::OK, headers, data)
}

async fn handler(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
	Ok(get_static_file(uri.clone()).await?)
}

async fn get_static_file(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
	let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
	// `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
	let server = MCSERVER.get().unwrap().read().await;

	match ServeDir::new(server.dir("mods")).oneshot(req).await {
		Ok(res) => Ok(res.map(boxed)),
		Err(err) => Err((
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("Something went wrong: {}", err),
		)),
	}
}

#[derive(Deserialize)]
struct GetFileQuery {
	zip: Option<bool>,
}

async fn get_mc_file(uri: Uri, _: Protected) -> Result<Response<BoxBody>, (StatusCode, String)> {
	let mut req = Request::builder().uri(uri.clone()).body(Body::empty()).unwrap();
	// `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`

	let server = MCSERVER.get().unwrap().read().await;
	let server_dir = server.dir("");
	drop(server);// free here to prevent write blocking
	if let Some(Ok(GetFileQuery { zip: Some(true) })) = uri.query().map(|it| serde_urlencoded::from_str::<GetFileQuery>(it)) {

		let zip = get_zip_file(server_dir.join(uri.path().trim_start_matches('/'))).await.unwrap();
		let zip = zip.strip_prefix(&server_dir).unwrap();
		req = Request::builder().uri(Uri::from_str(format!("/{}", zip.to_string_lossy()).as_str()).unwrap()).body(Body::empty()).unwrap();
	}

	match ServeDir::new(server_dir).oneshot(req).await {
		Ok(res) => Ok(res.map(boxed)),
		Err(err) => Err((
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("Something went wrong: {}", err),
		)),
	}
}


#[derive(Deserialize)]
struct ListFile {
	path: String,
}

async fn list_mc_file(Json(ListFile { path }): Json<ListFile>, _: Protected) -> impl IntoResponse {
	let server = MCSERVER.get().unwrap().read().await;
	let current = server.list_dir(path.as_str()).await;
	(StatusCode::OK, Json(current))
}

async fn update_mc_file(mut multipart: extract::Multipart, _: Protected) -> impl IntoResponse {
	let server = MCSERVER.get().unwrap().read().await;
	let root = server.dir("").canonicalize().unwrap();
	drop(server);
	while let Ok(Some(field)) = multipart.next_field().await {
		let name = field.name().unwrap().trim_start_matches(&['/', '.']);
		let target = root.join(name);
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
	"Ok"
}

#[derive(Deserialize)]
struct RemoveFile {
	#[serde(default)]
	paths: Vec<String>,
}

async fn rm_mc_file(Json(RemoveFile { paths }): Json<RemoveFile>, _: Protected) -> (StatusCode, Json<Vec<String>>) {
	if paths.is_empty() {
		(StatusCode::PARTIAL_CONTENT, Json(Vec::new()))
	} else {
		let mut removed: Vec<String> = Vec::with_capacity(paths.len());
		let server = MCSERVER.get().unwrap().read().await;
		let root = server.dir("").canonicalize().unwrap();
		drop(server);

		for file in paths {
			let target = root.join(file.as_str());
			println!("{:?}", target);
			if target.exists() {
				if target.is_dir() {
					if let Ok(_) = remove_dir_all(target).await {
						removed.push(file);
					};
				} else {
					if let Ok(_) = remove_file(target).await {
						removed.push(file);
					}
				}
			}
		}

		(StatusCode::OK, Json(removed))
	}
}

async fn get_web_file(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
	let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
	match ServeDir::new("web").oneshot(req).await {
		Ok(res) => Ok(res.map(boxed)),
		Err(err) => Err((
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("Something went wrong: {}", err),
		)),
	}
}

async fn shutdown(_: Protected) -> impl IntoResponse {
	let server = MCSERVER.get().unwrap().read().await;
	server.shutdown_in_place().await.ok();
	//std::process::exit(0);
	"Ok"
}

async fn kill(_: Protected) -> impl IntoResponse {
	let server = MCSERVER.get().unwrap().read().await;
	server.kill().await.ok();
	//std::process::exit(0);
	"Ok"
}

async fn update_config(config: MinecraftServerConfig) {
	debug!("updating config");
	let mut cfg = SERVER_CONFIG.get().unwrap().write().await;
	*cfg = config;
	drop(cfg); // unlock
}

async fn restart_server() -> Result<()> {
	let server = MCSERVER.get().unwrap().read().await;
	if let Ok(config) = server.create_config().await {
		update_config(config).await;
	}

	debug!("restarting server");
	server.restart_in_place().await.unwrap();
	Ok(())
}

async fn restart(_: Protected) -> impl IntoResponse {
	restart_server().await.ok();
	"Ok"
}

async fn update_cfg(Json(payload): Json<ForgeInfo>, _: Protected) -> impl IntoResponse {
	let server = MCSERVER.get().unwrap().read().await;
	update_config(server.update_forge_cfg(payload).await.unwrap()).await;
	"Ok"
}

async fn status() -> impl IntoResponse {
	let server = MCSERVER.get().unwrap().read().await;
	format!("{:?}", server.status().await)
}

async fn update(mut multipart: extract::Multipart, _: Protected) -> impl IntoResponse {
	while let Ok(Some(field)) = multipart.next_field().await {
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

				let server = MCSERVER.get().unwrap().read().await;
				let mc_server = server;

				let server_status = mc_server.status().await;

				let mod_info = MinecraftMod::new(&tmp).await.unwrap();

				let sconfig = SERVER_CONFIG.get().unwrap().read().await;
				let old = sconfig.mods.iter().find(|it| it.name == mod_info.name);
				let mod_dir = mc_server.dir("mods");
				if let Some(mcmod) = old {
					let old_file = mod_dir.join(mcmod.file_name.as_str());
					remove_file(&old_file).await.ok();
				}
				// deadlock
				drop(sconfig);

				rename(tmp, mod_dir.join(filename.as_str())).await.ok();

				// don't restart if server is gracefully stopped
				if server_status != STOPPED {
					tokio::spawn(async {
						if let Err(e) = restart_server().await {
							warn!("{}", e);
						};
					});
				} else {
					update_config(mc_server.create_config().await.unwrap()).await;
				}
				return (StatusCode::OK, Json(Some(mod_info)));
			}
		}
	}
	(StatusCode::OK, Json(None))
}

#[derive(Serialize)]
struct Status {
	ok: bool,
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