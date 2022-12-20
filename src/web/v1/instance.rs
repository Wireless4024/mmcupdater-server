use axum::extract::Query;
use axum::Router;
use axum::routing::get;
use serde::Deserialize;
use tracing::log::debug;

use crate::instance::mc_instance::McInstance;
use crate::manager::instance_manager::InstanceManagerExt;
use crate::util::errors::{HttpResult, ResponseResult};
use crate::web::authentication::Authorization;

pub fn build() -> Router {
	debug!("Configuring instance routes");
	Router::new()
		.route("/", get(all))
		.route("/info", get(info))
}

async fn all(m: InstanceManagerExt, _: Authorization) -> ResponseResult<Vec<String>> {
	let manager = m.read().await;
	HttpResult::success(manager.names().into_iter().map(|it| it.to_string()).collect::<Vec<_>>())
}

#[derive(Deserialize)]
struct InstanceName {
	name: String,
}

async fn info(Query(InstanceName { name }): Query<InstanceName>, m: InstanceManagerExt, _: Authorization) -> ResponseResult<McInstance> {
	let manager = m.read().await;
	match manager.find(&name) {
		None => {
			HttpResult::err("Not found")
		}
		Some(it) => {
			let it = it.read().await;
			HttpResult::success(it.clone())
		}
	}
}