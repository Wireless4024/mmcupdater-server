use axum::{Json, Router};
use axum::response::IntoResponse;
use axum::routing::get;
use tracing::log::debug;

use crate::info::DetailedInfo;
use crate::util::errors::{HttpResult, ResponseResult};

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
	Json(DetailedInfo::default())
}

async fn err() -> ResponseResult<&'static str, &'static str> {
	HttpResult::err("hello")
}

async fn success() -> ResponseResult<&'static str> {
	HttpResult::success("Ayyooo")
}
