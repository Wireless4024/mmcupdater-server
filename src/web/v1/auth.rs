use axum::{Json, Router};
use axum::extract::Query;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use tracing::log::debug;

use crate::db::DB;
use crate::entity::user::{PasswordSignResult, User};
use crate::util::config::get_config;
use crate::util::errors::HttpResult;
use crate::util::time::timestamp_minute;
use crate::web::authentication::Authorization;

pub fn build() -> Router {
	debug!("Configuring auth routes");
	Router::new()
		.route("/login", post(login))
		.route("/refresh", get(refresh_token))
		.route("/logout", get(logout))
		.route("/ping", get(ping))
}

#[derive(Deserialize)]
struct LoginPayload {
	username: String,
	password: String,
	#[serde(default)]
	set: bool,
}

const LOGIN_FAIL: (StatusCode, Json<HttpResult<String, &str>>) = (StatusCode::FORBIDDEN, HttpResult::err_raw("auth.invalid"));
const LOGIN_TOO_MANY: (StatusCode, Json<HttpResult<String, &str>>) = (StatusCode::FORBIDDEN, HttpResult::err_raw("auth.too_many"));
const LOGIN_NEED_RESET: (StatusCode, Json<HttpResult<String, &str>>) = (StatusCode::FORBIDDEN, HttpResult::err_raw("auth.need_reset"));

async fn login(db: DB,
               Json(LoginPayload { username, password, set }): Json<LoginPayload>) -> Response {
	let repo = db.repo_with_cache::<User>();
	let mut user = User::default();
	user.username = username;
	match repo.get_by(&["username"], &user).await {
		None => { LOGIN_FAIL.into_response() }
		Some(mut user) => {
			let pass_verify = user.check_pass_and_sign(password.into_bytes()).await;
			repo.update_minimal_owned(user).await.ok();
			match pass_verify {
				PasswordSignResult::NeedReset => {
					LOGIN_NEED_RESET.into_response()
				}
				PasswordSignResult::TooManyAttempt(_) => {
					// TODO: try to response this message
					LOGIN_TOO_MANY.into_response()
				}
				PasswordSignResult::Invalid => {
					LOGIN_FAIL.into_response()
				}
				PasswordSignResult::Valid(jwt) => {
					if set {
						let cfg = get_config().await;

						let max_age = cfg.http.jwt.valid_time * 60;
						drop(cfg);
						let mut resp = (StatusCode::OK, HttpResult::success_raw("Ok")).into_response();
						resp.headers_mut().insert("set-cookie",
						                          HeaderValue::from_str(
							                          &format!("authorization={jwt}; Path=/; Max-Age={max_age}; Secure; SameSite=None; HttpOnly")
						                          ).unwrap());
						resp
					} else {
						(StatusCode::OK, HttpResult::success_raw(jwt)).into_response()
					}
				}
			}
		}
	}
}

#[derive(Deserialize)]
struct Set {
	#[serde(default = "default_set")]
	set: bool,
}

fn default_set() -> bool {
	true
}

impl Default for Set {
	fn default() -> Self {
		Self {
			set: true
		}
	}
}

async fn refresh_token(Query(Set { set }): Query<Set>, jwt: Authorization) -> impl IntoResponse {
	let jwt = jwt.refresh().await;
	if set {
		let max_age = {
			let exp_mins = get_config().await.http.jwt.valid_time;
			(timestamp_minute() + exp_mins) * 60
		};
		let mut resp = (StatusCode::OK, HttpResult::success_raw("Ok")).into_response();
		resp.headers_mut().insert("set-cookie",
		                          HeaderValue::from_str(
			                          &format!("authorization={jwt}; Path=/; Max-Age={max_age}; Secure; SameSite=None; HttpOnly")
		                          ).unwrap());
		resp
	} else {
		(StatusCode::OK, HttpResult::success_raw(jwt)).into_response()
	}
}

async fn logout(_: Authorization) -> impl IntoResponse {
	let mut resp = (StatusCode::OK, HttpResult::success_raw("Ok")).into_response();
	resp.headers_mut().insert("set-cookie", HeaderValue::from_str(
		"authorization=; Path=/; Max-Age=0; Secure; SameSite=None; HttpOnly"
	).unwrap());
	resp
}

async fn ping(_: Authorization) -> impl IntoResponse {
	(StatusCode::OK, "Hi")
}