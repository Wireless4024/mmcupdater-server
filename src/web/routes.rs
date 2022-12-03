use axum::{Json, Router};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::info::GlobalInfo;
use crate::web::v1::get_v1;

pub fn build_route(route: Router) -> Router {
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
	route.route("/api", get(api_info))
		.nest("/api/v1", get_v1())
		.fallback(not_found)
}

async fn api_info() -> impl IntoResponse {
	Json(GlobalInfo::default())
}

async fn not_found() -> impl IntoResponse {
	(StatusCode::NOT_FOUND, r#"{"success":false,"message":"The resource you are looking for is unavailable"}"#)
}