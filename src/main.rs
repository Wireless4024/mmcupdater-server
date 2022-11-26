#![feature(str_split_as_str)]
#![feature(io_error_more)]
#![forbid(unsafe_code)]

extern crate core;

use std::str::FromStr;

use anyhow::Result;
use axum::extract::FromRequest;
use axum::response::IntoResponse;
use axum::routing::{delete, post, put};
use bstr::ByteSlice;
use futures::StreamExt;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::util::ServiceExt;
use tracing::info;

use crate::jar_scanner::get_manifest;
use crate::manager::instance_manager::InstanceManager;
use crate::util::{config, logger};
use crate::util::java::JavaManager;
use crate::web::http;

mod file_scanner;
mod file_info;
mod schema;
mod jar_scanner;
mod util;
mod instance;
mod manager;
mod macros;
mod mc;
mod web;

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
	
	let _guard = logger::init();
	info!("starting MMC Updater server {}", env!("CARGO_PKG_VERSION"));
	config::load_config().await;
//	println!("{:?}", get_manifest("mc/mods/Quark-r2.4-321.jar").await?);;

	JavaManager::scan().await?;
	//JavaManager::download_version(8).await?;
	//JavaManager::try_purge_jdk("java_runtime/java8").await?;
	println!("{:?}", JavaManager::versions().await);
	let mut manager = InstanceManager::new();
	//println!("{:?}", ModType::Vanilla.versions(&Client::new()).await);
	manager.init().await?;
	//manager.restart("mc1234").await;
	//let instance = manager.new_instance("mc1234", "1.16.5", ModType::Purpur).await.expect("TODO: panic message");
	/*{
		let res = manager.with_instance("mc1234", |mut it| {
			async move {
				it.restart_in_place().await??;
				Result::<()>::Ok(())
			}
		}).await.unwrap()?;
	}*/
	println!("{:?}", manager.names());
	http::init(manager.into_extension()).await;
	Ok(())
}