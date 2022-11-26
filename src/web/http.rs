use std::net::SocketAddr;

use axum::{Router, Server};
use axum::routing::IntoMakeService;
use hyper::server::conn::AddrIncoming;
use tower_http::cors::CorsLayer;

use crate::manager::instance_manager::InstanceManagerExt;
use crate::util::config::get_config;

pub async fn init(manager: InstanceManagerExt) -> Server<AddrIncoming, IntoMakeService<Router>> {
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


	let defaut_addr = SocketAddr::from((if cfg.http.expose { [0, 0, 0, 0] } else { [127, 0, 0, 1] }, cfg.http.port));

	let mut builder = axum::Server::bind(&defaut_addr)
		.tcp_nodelay(true);
	if cfg.http.secure {
		//builder.cer
	}
	builder.serve(app.into_make_service())
}
// https://github.com/tokio-rs/axum/discussions/1063
// https://github.com/tokio-rs/axum/blob/main/examples/tls-rustls/src/main.rs