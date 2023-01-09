use axum::{Json, Router};
use axum::extract::Path;
use axum::routing::{get, post};
use serde::Deserialize;
use tracing::log::debug;

use crate::instance::mc_instance::ModType;
use crate::manager::instance_manager::InstanceManagerExt;
use crate::util::errors::{ResponseResult, ResultBase};
use crate::util::errors::rest::{conflict, created, got, no_content, not_found, Resp};
use crate::web::authentication::Authorization;

pub fn build() -> Router {
	debug!("Configuring instance routes");
	Router::new()
		.route("/", get(all))
		.route("/:name", get(info).delete(delete).post(create))
}

#[derive(Deserialize)]
struct InstancePath {
	name: String,
}

async fn all(m: InstanceManagerExt, _: Authorization) -> ResponseResult<Vec<String>> {
	let manager = m.read().await;
	ResultBase::success(manager.names())
}

#[derive(Deserialize)]
struct InstanceName {
	name: String,
}

async fn info(Path(InstancePath { name }): Path<InstancePath>, m: InstanceManagerExt, _: Authorization) -> Resp {
	let manager = m.read().await;
	match manager.find(&name) {
		Some(it) => {
			let it = it.read().await;
			got(it.clone())
		}
		None => {
			not_found()
		}
	}
}

async fn delete(Path(InstancePath { name }): Path<InstancePath>, m: InstanceManagerExt, _: Authorization) -> Resp {
	let manager = m.write().await;
	match manager.remove_instance(&name).await? {
		Some(_) => {
			no_content()
		}
		None => {
			not_found()
		}
	}
}

#[derive(Deserialize)]
struct InstanceCreate {
	version: String,
	typ: ModType,
}

async fn create(Path(InstancePath { name }): Path<InstancePath>,
                m: InstanceManagerExt,
                _: Authorization,
                Json(InstanceCreate { version, typ }): Json<InstanceCreate>,
) -> Resp {
	let manager = m.read().await;
	match manager.find(&name) {
		None => {
			let instance = manager.new_instance(&name, &version, typ).await?;
			let it = instance.read().await;
			created(it.clone())
		}
		Some(_) => {
			conflict()
		}
	}
}