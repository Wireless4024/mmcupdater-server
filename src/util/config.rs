use std::{env, mem};
use std::fs::File;

use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio::task::{JoinError, spawn_blocking};
use tracing::{error, info};

use crate::util::fs::create_if_not_existed;

static DEFAULT_CONFIG_YML: &str = include_str!("../resources/dummy_config.yml");
static CONFIG: RwLock<ConfigRoot> = RwLock::const_new(ConfigRoot::const_default());

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ConfigRoot {
	pub http: HttpConfig,
	pub monitor: MonitorConfig,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct HttpConfig {
	/// Accept client from anywhere? leave it to false if you want to reverse-proxy to this service
	#[serde(default)]
	pub expose: bool,
	/// Http listen port
	#[serde(default = "default_port")]
	pub port: u16,
	/// Listen via unix socket (linux only)
	#[serde(default)]
	pub socket: String,
	/// Enable http/2 and ssl
	#[serde(default)]
	pub secure: bool,
	/// certificate file (full-chain)
	#[serde(default)]
	pub cert_file: Option<String>,
	/// certificate key (private key)
	#[serde(default)]
	pub cert_key: Option<String>,
}

fn default_port() -> u16 { 8181 }

pub async fn get_config() -> RwLockReadGuard<'static, ConfigRoot> {
	CONFIG.read().await
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct MonitorConfig {
	#[serde(default)]
	pub prometheus: PrometheusConfig,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PrometheusConfig {
	#[serde(default)]
	pub enable: bool,
}

impl ConfigRoot {
	pub const fn const_default() -> Self {
		Self {
			http: HttpConfig {
				expose: false,
				port: 8181,
				socket: String::new(),
				secure: false,
				cert_file: None,
				cert_key: None,
			},
			monitor: MonitorConfig {
				prometheus: PrometheusConfig {
					enable: false,
				},
			},
		}
	}
}

pub async fn load_config() {
	info!("loading config..");
	if let Ok(file) = File::open("config.yml") {
		let cfg = spawn_blocking(|| {
			let config: Value = serde_yaml::from_reader(file).expect("Load config");
			let config = env_to_yml(config);
			serde_yaml::from_value::<ConfigRoot>(config)
		}).await;
		if let Ok(Ok(res)) = cfg {
			*CONFIG.write().await = res;
		} else {
			error!("Failed to load config; using default config")
		}
	} else {
		create_if_not_existed("config.yml", DEFAULT_CONFIG_YML)
			.await
			.expect("generate config file");
	}
}

pub async fn save_config() -> Result<(), JoinError> {
	info!("saving config..");
	let cfg = get_config().await;
	spawn_blocking(move || {
		if let Ok(file) = File::create("config.yml") {
			serde_yaml::to_writer(file, &*cfg).ok();
		}
	}).await
}

pub async fn modify<V: Serialize>(path: impl Into<String>, val: V) {
	let mut cfg = CONFIG.write().await;
	let value = serde_yaml::to_value(mem::take(&mut *cfg)).unwrap();
	if let Value::Mapping(mut map) = value {
		map.insert(serde_yaml::to_value(path.into()).unwrap(), serde_yaml::to_value(val).unwrap());
		*cfg = serde_yaml::from_value(Value::Mapping(map)).unwrap();
	}
}

fn env_to_yml(base: Value) -> Value {
	let mut value = if let Value::Mapping(value) = base {
		value
	} else {
		Mapping::new()
	};

	for (k, v) in env::vars() {
		if k.starts_with('_') {
			value.insert(Value::String(k.strip_prefix('_').unwrap().replace('_', ".")), Value::String(v));
		}
	}

	Value::Mapping(value)
}