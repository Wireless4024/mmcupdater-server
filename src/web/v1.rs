use axum::response::IntoResponse;
use axum::Router;
use axum::routing::get;
use tokio::task::spawn_blocking;
use tracing::log::debug;

use crate::info::DetailedInfo;
use crate::util::errors::{ResultBase, ResponseResult};

mod auth;
mod instance;
mod user;

pub fn get_v1() -> Router {
	debug!("Configuring v1 routes");
	Router::new()
		.nest("/auth", auth::build())
		.nest("/instance", instance::build())
		.nest("/user", user::build())
		.route("/err", get(err))
		.route("/success", get(success))
		.route("/info", get(info))
}

async fn info() -> impl IntoResponse {
	ResultBase::success_raw(spawn_blocking(DetailedInfo::default).await.unwrap())
}

async fn err() -> ResponseResult<&'static str, &'static str> {
	ResultBase::err("hello")
}

async fn success() -> ResponseResult<&'static str> {
	ResultBase::success("Ayyooo")
}
