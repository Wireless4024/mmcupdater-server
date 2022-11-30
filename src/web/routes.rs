use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router};
use axum::routing::get;

use crate::util::errors::{ErrorWrapper, HttpResult, ResponseResult};

const API_VERSION: usize = 1;

pub fn build_route(route: Router) -> Router {
	let api_router = Router::new()
		.route("/err", get(err))
		.route("/success", get(success));
	/* example route
	.route("/stop", get(shutdown))
		.route("/kill", get(kill))
		.route("/status", get(status))
		.route("/restart", get(restart))
		.route("/update", post(update))
		.route("/update_cfg", post(update_cfg))
		.nest("/mc/file", get(get_mc_file))
		.route("/mc/file", post(list_mc_file))
		.route("/mc/file", put(update_mc_file))
		.route("/mc/file", delete(rm_mc_file))*/
	route.nest(&format!("/api/v{API_VERSION}"), api_router)
		.fallback(not_found)
}

async fn err() -> Result<Json<HttpResult<&'static str>>,ErrorWrapper> {
	HttpResult::err("hello")
}

async fn success() -> ResponseResult<&'static str> {
	HttpResult::success("Ayyooo")
}

async fn not_found() -> impl IntoResponse {
	(StatusCode::NOT_FOUND, r#"{"success":false,"message":"The resource you are looking for is unavailable"}"#)
}