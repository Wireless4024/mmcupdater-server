use axum::Router;
use axum::routing::get;
use serde_json::Value;
use tracing::log::debug;

use base::ser_ref::ToJsonValue;

use crate::db::DB;
use crate::entity::user::User;
use crate::util::errors::{ResultBase, ResponseResult};
use crate::web::authentication::Authorization;

pub fn build() -> Router {
	debug!("Configuring user routes");
	Router::new()
		.route("/", get(me))
}

async fn me(db: DB, Authorization(inner): Authorization) -> ResponseResult<Value> {
	let uid = inner.id;
	let repo = db.repo_with_cache::<User>();
	let user = repo.get(uid).await.unwrap();
	ResultBase::success(user.to_json())
}