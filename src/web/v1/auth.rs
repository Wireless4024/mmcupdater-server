use axum::{Json, Router};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use cookie::SameSite;
use cookie::time::Duration;
use serde::Deserialize;
use tower_cookies::{Cookie, Cookies};

use crate::db::DB;
use crate::util::config::get_config;
use crate::util::errors::HttpResult;
use crate::web::authentication::{Authorization, PasswordSignResult};
use crate::web::User;

pub fn build() -> Router {
	Router::new()
		.route("/login", post(login))
		.route("/ping", get(ping))
}

#[derive(Deserialize)]
struct LoginPayload {
	username: String,
	password: String,
	#[serde(default)]
	set: bool,
}

const LOGIN_FAIL: (StatusCode, Json<HttpResult<String, &str>>) = (StatusCode::FORBIDDEN, HttpResult::err_raw("password.invalid"));
const LOGIN_TOO_MANY: (StatusCode, Json<HttpResult<String, &str>>) = (StatusCode::FORBIDDEN, HttpResult::err_raw("password.too_many"));
const LOGIN_NEED_RESET: (StatusCode, Json<HttpResult<String, &str>>) = (StatusCode::FORBIDDEN, HttpResult::err_raw("password.need_reset"));

async fn login(db: DB,
               cookie: Cookies,
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

						cookie.add(Cookie::build("authorization", jwt)
							.secure(cfg.http.secure)
							.http_only(true)
							.same_site(if cfg.http.jwt.same_site { SameSite::Strict } else { SameSite::None })
							.max_age(Duration::minutes(cfg.http.jwt.valid_time))
							.finish());
						drop(cfg);

						(StatusCode::OK, HttpResult::success_raw("Ok")).into_response()
					} else {
						(StatusCode::OK, HttpResult::success_raw(jwt)).into_response()
					}
				}
			}
		}
	}
}

async fn ping(_: Authorization) -> impl IntoResponse {
	(StatusCode::OK, "Hi")
}