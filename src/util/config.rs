use std::{env, mem};
use std::borrow::Cow;
use std::fs::File;

use axum::http::{HeaderValue, Method};
use jsonwebtoken::Algorithm;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio::task::{JoinError, spawn_blocking};
use tower_http::cors::{AllowHeaders, CorsLayer};
use tracing::{error, info};

use crate::util::fs::create_if_not_existed;

static DEFAULT_CONFIG_YML: &str = include_str!("../resources/dummy_config.yml");
static CONFIG: RwLock<ConfigRoot> = RwLock::const_new(ConfigRoot::const_default());

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ConfigRoot {
	#[serde(default)]
	pub http: HttpConfig,
	#[serde(default)]
	pub monitor: MonitorConfig,
	#[serde(default)]
	pub security: Security,
}

#[derive(Serialize, Deserialize, Debug)]
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

	/// Proxy root location to new location (used in development)
	#[serde(default = "default_root_proxy")]
	pub root_proxy: Option<String>,

	/// config related to jwt
	#[serde(default)]
	pub jwt: JwtConfig,

	/// config related to jwt
	#[serde(default)]
	pub cors: Cors,
}

const fn default_port() -> u16 { 8181 }

#[cfg(not(debug_assertions))]
fn default_root_proxy() -> Option<String> { None }

#[cfg(debug_assertions)]
fn default_root_proxy() -> Option<String> { Some("http://127.0.0.1:5173".to_string()) }

impl Default for HttpConfig {
	fn default() -> Self {
		Self {
			expose: false,
			port: default_port(),
			socket: String::new(),
			secure: false,
			cert_file: None,
			cert_key: None,
			root_proxy: default_root_proxy(),
			jwt: Default::default(),
			cors: Default::default(),
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JwtConfig {
	/// Jwt algorithm (modify this if you need compatibility or security)
	#[serde(default = "default_jwt_algo")]
	pub algo: Algorithm,
	/// Path to public key for jwt
	#[serde(default)]
	pub enc_key: String,
	/// Path to private key for jwt
	#[serde(default)]
	pub dec_key: String,
	/// Valid time in minutes
	#[serde(default = "default_valid_time")]
	pub valid_time: u64,
	/// Expose token for cross site api use
	#[serde(default = "default_same_site")]
	pub same_site: bool,
}

impl Default for JwtConfig {
	fn default() -> Self {
		Self {
			algo: default_jwt_algo(),
			enc_key: String::new(),
			dec_key: String::new(),
			valid_time: default_valid_time(),
			same_site: default_same_site(),
		}
	}
}

const fn default_jwt_algo() -> Algorithm { Algorithm::RS256 }

const fn default_valid_time() -> u64 { 60 * 24 * 7 }

const fn default_same_site() -> bool { true }

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

#[derive(Serialize, Deserialize, Debug)]
pub struct Security {
	#[serde(default = "default_max_login_retry")]
	pub max_login_retry: i32,
	#[serde(default = "default_login_cool_down")]
	pub login_cool_down: u64,
}

impl Default for Security {
	fn default() -> Self {
		Self {
			max_login_retry: default_max_login_retry(),
			login_cool_down: default_login_cool_down(),
		}
	}
}

const fn default_max_login_retry() -> i32 { 15 }

const fn default_login_cool_down() -> u64 { 30 }

#[derive(Serialize, Deserialize, Debug)]
pub struct Cors {
	/// list of allowed methods send by cors header
	#[serde(default = "default_cors_methods")]
	methods: Cow<'static, [Cow<'static, str>]>,
	/// list of allowed origins send by cors header
	#[serde(default)]
	origins: Vec<String>,
}

const fn default_cors_methods() -> Cow<'static, [Cow<'static, str>]> {
	Cow::Borrowed([
		Cow::Borrowed("GET"),
		Cow::Borrowed("POST"),
		Cow::Borrowed("HEAD"),
		Cow::Borrowed("OPTION"),
	].as_slice())
}

impl Default for Cors {
	fn default() -> Self {
		Self {
			methods: default_cors_methods(),
			origins: vec![],
		}
	}
}

impl Cors {
	pub fn build(&self) -> CorsLayer {
		let mut cors = CorsLayer::new();
		for m in self.methods.iter() {
			cors = cors.allow_methods(Method::from_bytes(m.as_bytes()).unwrap());
		}
		for o in self.origins.iter() {
			cors = cors.allow_origin(HeaderValue::from_str(o).unwrap());
		}
		cors.allow_headers(AllowHeaders::mirror_request())
			.allow_credentials(true)
	}
}

impl ConfigRoot {
	pub const fn const_default() -> Self {
		Self {
			http: HttpConfig {
				expose: false,
				port: default_port(),
				socket: String::new(),
				secure: false,
				cert_file: None,
				cert_key: None,
				root_proxy: None,
				jwt: JwtConfig {
					algo: default_jwt_algo(),
					enc_key: String::new(),
					dec_key: String::new(),
					valid_time: default_valid_time(),
					same_site: default_same_site(),
				},
				cors: Cors {
					methods: default_cors_methods(),
					origins: Vec::new(),
				},
			},
			monitor: MonitorConfig {
				prometheus: PrometheusConfig {
					enable: false,
				},
			},
			security: Security {
				max_login_retry: default_max_login_retry(),
				login_cool_down: default_login_cool_down(),
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
		info!("config not found, generating new config file..");
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