use std::net::SocketAddr;
use std::path::PathBuf;

use axum::{Extension, Router};
use axum_server::tls_rustls::RustlsConfig;
use sqlx::Sqlite;
use tracing::{debug, info};

use crate::db::DbWrapper;
use crate::manager::instance_manager::InstanceManagerExt;
use crate::util::config::get_config;
use crate::util::errors::ErrorWrapper;
use crate::web::routes::build_route;

pub async fn init(manager: InstanceManagerExt, db: DbWrapper<Sqlite>) -> Result<(), ErrorWrapper> {
	let cfg = get_config().await;
	let app = build_route(Router::new());
	let app = app
		.layer(cfg.http.cors.build())
		.layer(manager)
		.layer(Extension(db));
	debug!("configuring http server");
	let defaut_addr = SocketAddr::from((if cfg.http.expose { [0, 0, 0, 0] } else { [127, 0, 0, 1] }, cfg.http.port));

	super::authentication::init().await?;
	info!("starting http service at port http{}://{:?}",if cfg.http.secure{"s"}else{""},defaut_addr);

	if cfg.http.secure {
		let config = RustlsConfig::from_pem_file(
			PathBuf::from(cfg.http.cert_file.as_ref().expect("Certificate file config")),
			PathBuf::from(cfg.http.cert_key.as_ref().expect("Certificate key config")),
		)
			.await?;
		axum_server::bind_rustls(defaut_addr, config)
			.serve(app.into_make_service()).await?
	} else {
		axum::Server::bind(&defaut_addr)
			.tcp_nodelay(true)
			.serve(app.into_make_service()).await?
	}
	Ok(())
}
// https://github.com/tokio-rs/axum/discussions/1063
// https://github.com/tokio-rs/axum/blob/main/examples/tls-rustls/src/main.rs