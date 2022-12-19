use axum::Router;
use axum::routing::get;
use tracing::log::debug;

use crate::manager::instance_manager::InstanceManagerExt;
use crate::util::errors::{HttpResult, ResponseResult};
use crate::web::authentication::Authorization;

pub fn build() -> Router {
	debug!("Configuring instance routes");
	Router::new()
		.route("/", get(all))
}

async fn all(m: InstanceManagerExt, _: Authorization) -> ResponseResult<Vec<String>> {
	let manager = m.read().await;
	HttpResult::success(manager.names().into_iter().map(|it| it.to_string()).collect::<Vec<_>>())
}