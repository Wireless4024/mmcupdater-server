#![feature(str_split_as_str)]
#![feature(io_error_more)]

extern crate core;

use anyhow::Result;
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
mod db;
mod entity;
mod cli;

fn main() -> Result<()> {
	cli::intercept();

	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.global_queue_interval(255)
		.build()?;

	let _guard = logger::init();

	rt.block_on(async {
		info!("starting MMC Updater server {}", GlobalInfo::VERSION);
		config::load_config().await;
		JavaManager::scan().await?;
		let mut manager = InstanceManager::new();
		manager.init().await?;
		let db = db::init().await?;
		http::init(manager.into_extension(), db).await?;
		Result::<()>::Ok(())
	})
}