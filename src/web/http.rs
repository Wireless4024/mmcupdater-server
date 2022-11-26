use std::net::SocketAddr;
use std::path::PathBuf;

use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use tower_http::cors::CorsLayer;
use tracing::{debug, info};

use crate::manager::instance_manager::InstanceManagerExt;
use crate::util::config::get_config;
use crate::util::errors::ErrorWrapper;

pub async fn init(manager: InstanceManagerExt) -> Result<(), ErrorWrapper> {
	let cfg = get_config().await;
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
		.layer(CorsLayer::permissive())
		.layer(manager);
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

	debug!("configuring http server");
	let defaut_addr = SocketAddr::from((if cfg.http.expose { [0, 0, 0, 0] } else { [127, 0, 0, 1] }, cfg.http.port));

	info!("starting http service at port {}",cfg.http.port);

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