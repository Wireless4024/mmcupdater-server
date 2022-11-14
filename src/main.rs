#![feature(str_split_as_str)]
#![feature(io_error_more)]
#![forbid(unsafe_code)]

extern crate core;

use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;

use anyhow::Result;
use axum::{async_trait, Router};
use axum::extract::{FromRequest, RequestParts};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, post, put};
use futures::StreamExt;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::util::ServiceExt;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::jar_scanner::get_manifest;

mod config;
mod file_scanner;
mod file_info;
mod schema;
mod jar_scanner;
mod util;
mod instance;
mod manager;

async fn init() -> Result<()> {
	/*let mut default_config_file = File::open("default_server.json").await?;
	let mut data = String::new();
	default_config_file.read_to_string(&mut data).await?;
	info!("Loading default server");

	let cfg: Config = serde_json::from_str(data.as_str()).unwrap();

	let config = cfg.minecraft.create_config().await.unwrap();

	SERVER_CONFIG.set(RwLock::new(config)).ok();
	info!("Building minecraft instance");
	let server = cfg.minecraft.build().unwrap();
	MCSERVER.set(RwLock::new(server)).ok();
*/
	Result::<()>::Ok(())
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
		/*.route("/stop", get(shutdown))
		.route("/kill", get(kill))
		.route("/status", get(status))
		.route("/restart", get(restart))
		.route("/update", post(update))
		.route("/update_cfg", post(update_cfg))
		.nest("/mc/file", get(get_mc_file))
		.route("/mc/file", post(list_mc_file))
		.route("/mc/file", put(update_mc_file))
		.route("/mc/file", delete(rm_mc_file))*/
		.layer(CorsLayer::permissive());
	let mut app = app;
	//.nest("/mods", get(handler))
	//.route("/config.json", get(config));

	/*	if let Ok(mut serve) = env::var("serve_config") {
			serve.make_ascii_lowercase();
			match serve.as_str() {
				"y" | "1" | "true" => {
					app = app.route("/config.zip", get(config_dir));
				}
				_ => {}
			}
		}*/

	if Path::new("web").exists() {
		info!("found web folder adding route to it");
		//	app = app.nest("/web", get(get_web_file))
	}
	init().await?;

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
/*
async fn config() -> impl IntoResponse {
	(StatusCode::OK, Json((*get_server().await.read().await).clone()))
}*/
/*
async fn config_dir() -> impl IntoResponse {
	let server = MCSERVER.get().unwrap().read().await;

	let data = server.get_config_zip().await.unwrap();
	let mut headers = HeaderMap::new();
	headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/zip"));
	(StatusCode::OK, headers, data)
}*/

#[derive(Deserialize)]
struct GetFileQuery {
	zip: Option<bool>,
}

#[derive(Deserialize)]
struct ListFile {
	path: String,
}/*

async fn list_mc_file(Json(ListFile { path }): Json<ListFile>, _: Protected) -> impl IntoResponse {
	let server = MCSERVER.get().unwrap().read().await;
	let current = server.list_dir(path.as_str()).await;
	(StatusCode::OK, Json(current))
}*/

#[derive(Serialize)]
struct Status {
	ok: bool,
}

pub(crate) struct Protected;

#[async_trait]
impl<B: Send> FromRequest<B> for Protected {
	type Rejection = AuthError;

	async fn from_request(req: &mut RequestParts<B>) -> std::result::Result<Self, Self::Rejection> {
		let headers = req.headers();
		if let Some(key) = headers.get("Authorization") {
			if key.to_str().unwrap() == std::env::var("auth").unwrap().as_str() {
				return Ok(Protected);
			}
		}
		Err(AuthError)
	}
}

struct AuthError;

impl IntoResponse for AuthError {
	fn into_response(self) -> axum::response::Response {
		(StatusCode::UNAUTHORIZED, "").into_response()
	}
}