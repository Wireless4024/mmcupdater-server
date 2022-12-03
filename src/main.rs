#![feature(str_split_as_str)]
#![feature(io_error_more)]
#![forbid(unsafe_code)]

extern crate core;

use std::str::FromStr;

use anyhow::__private::kind::AdhocKind;
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

use crate::info::GlobalInfo;
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
mod info;

#[tokio::main]
async fn main() -> Result<()> {
	dotenv::dotenv().ok();

	let _guard = logger::init();
	info!("starting MMC Updater server {}", GlobalInfo::VERSION);
	config::load_config().await;
	JavaManager::scan().await?;

	let mut manager = InstanceManager::new();
	manager.init().await?;
	http::init(manager.into_extension()).await?;
	Ok(())
}