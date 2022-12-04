use futures::join;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::Private;
use openssl::rsa::Rsa;
use rand::RngCore;
use tokio::fs::metadata;
use tracing::{debug, info};

pub use cred::{PasswordCheck, User};
use jwt::{DECODE_KEY, DEFAULT_PRI, DEFAULT_PUB, ENCODE_KEY};

use crate::util::config::get_config;

pub async fn init() -> std::io::Result<()> {
	debug!("configuring authentication service");
	let cfg = get_config().await;
	let jwt = &cfg.http.jwt;
	if (jwt.dec_key.is_empty() || jwt.enc_key.is_empty())
		&& (metadata(DEFAULT_PRI).await.is_err() && metadata(DEFAULT_PUB).await.is_err()) {
		info!("generating new jwt key");
		// generate and load jwt key here
		match jwt.algo {
			Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
				let mut secret = match jwt.algo {
					Algorithm::HS256 => vec![0u8; 256],
					Algorithm::HS384 => vec![0u8; 384],
					_ => vec![0u8; 512]
				};
				let mut rng = rand::thread_rng();
				rng.fill_bytes(&mut secret);
				tokio::fs::write(DEFAULT_PRI, &secret).await?;
				tokio::fs::write(DEFAULT_PUB, &secret).await?;
			}
			Algorithm::ES256 | Algorithm::ES384 => {
				let group = if matches!(jwt.algo,Algorithm::ES256) {
					EcGroup::from_curve_name(Nid::ECDSA_WITH_SHA256).unwrap()
				} else {
					EcGroup::from_curve_name(Nid::ECDSA_WITH_SHA384).unwrap()
				};
				let key: EcKey<Private> = EcKey::<Private>::generate(&group).unwrap();
				tokio::fs::write(DEFAULT_PRI, key.private_key_to_pem().unwrap()).await?;
				tokio::fs::write(DEFAULT_PUB, key.public_key_to_pem().unwrap()).await?;
			}
			Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 |
			Algorithm::PS256 | Algorithm::PS384 | Algorithm::PS512 => {
				let key: Rsa<Private> = Rsa::generate(match jwt.algo {
					Algorithm::RS256 | Algorithm::PS256 => 2048,
					Algorithm::RS384 | Algorithm::PS384 => 3072,
					_ => 4096
				}).unwrap();
				tokio::fs::write(DEFAULT_PRI, key.private_key_to_pem().unwrap()).await?;
				tokio::fs::write(DEFAULT_PUB, key.public_key_to_pem().unwrap()).await?;
			}
			Algorithm::EdDSA => {
				panic!("Generate key is not support by this algorithm");
			}
		}
	}
	{
		debug!("loading jwt key");
		let enc_key = tokio::fs::read(
			Some(&jwt.enc_key)
				.and_then(|it| if it.is_empty() { None } else { Some(it.as_str()) })
				.unwrap_or(DEFAULT_PUB)).await?;
		let dec_key = tokio::fs::read(
			Some(&jwt.dec_key)
				.and_then(|it| if it.is_empty() { None } else { Some(it.as_str()) })
				.unwrap_or(DEFAULT_PRI)).await?;

		match jwt.algo {
			Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
				let e = ENCODE_KEY.get_or_init(|| async { EncodingKey::from_secret(&enc_key) });
				let d = DECODE_KEY.get_or_init(|| async { DecodingKey::from_secret(&dec_key) });
				join!(e, d);
			}
			Algorithm::ES256 | Algorithm::ES384 => {
				let e = ENCODE_KEY.get_or_init(|| async { EncodingKey::from_ec_pem(&enc_key).expect("load encoding key") });
				let d = DECODE_KEY.get_or_init(|| async { DecodingKey::from_ec_pem(&dec_key).expect("load decoding key") });
				join!(e, d);
			}
			Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 |
			Algorithm::PS256 | Algorithm::PS384 | Algorithm::PS512 => {
				let e = ENCODE_KEY.get_or_init(|| async { EncodingKey::from_rsa_pem(&enc_key).expect("load encoding key") });
				let d = DECODE_KEY.get_or_init(|| async { DecodingKey::from_rsa_pem(&dec_key).expect("load decoding key") });
				join!(e, d);
			}
			Algorithm::EdDSA => {
				let e = ENCODE_KEY.get_or_init(|| async { EncodingKey::from_ed_pem(&enc_key).expect("load encoding key") });
				let d = DECODE_KEY.get_or_init(|| async { DecodingKey::from_ed_pem(&dec_key).expect("load decoding key") });
				join!(e, d);
			}
		}
	}
	Ok(())
}

mod jwt {
	use async_trait::async_trait;
	use axum::extract::FromRequest;
	use axum::http::{Request, StatusCode};
	use jsonwebtoken::{decode, DecodingKey, EncodingKey, Validation};
	use serde::{Deserialize, Serialize};
	use tokio::sync::OnceCell;

	use crate::util::config::get_config;
	use crate::util::errors::ErrorWrapper;

	pub(crate) static ENCODE_KEY: OnceCell<EncodingKey> = OnceCell::const_new();
	pub(crate) static DECODE_KEY: OnceCell<DecodingKey> = OnceCell::const_new();

	pub(crate) static DEFAULT_PRI: &str = "jwt.key";
	pub(crate) static DEFAULT_PUB: &str = "jwt.pub";

	#[derive(Serialize, Deserialize)]
	pub struct JwtContent {
		id: i32,
	}

	pub struct Authorization(pub JwtContent);

	#[async_trait]
	impl<B: Send + 'static, S: Send + Sync> FromRequest<S, B> for Authorization {
		type Rejection = ErrorWrapper;

		async fn from_request(req: Request<B>, _state: &S) -> Result<Self, Self::Rejection> {
			let headers = req.headers();
			if let Some(key) = headers.get("Authorization") {
				if let Ok(v) = key.to_str() {
					let jwt = v.split("Bearer ").nth(1);
					if let Some(jwt) = jwt {
						let jwt_algo = {
							get_config().await.http.jwt.algo
						};
						if let Ok(claim) = decode::<JwtContent>(
							jwt,
							DECODE_KEY.get().unwrap(),
							&Validation::new(jwt_algo)) {
							return Ok(Authorization(claim.claims));
						};
					}
				}
			}

			Err(ErrorWrapper::custom(StatusCode::UNAUTHORIZED, "Unauthorized"))
		}
	}
}

mod cred {
	use std::ops::Deref;

	use argon2::Config;
	use hashbrown::HashSet;
	use rand::RngCore;
	use serde::{Deserialize, Serialize};
	use crate::mod_field;

	use crate::util::modification::{ModificationTracker};

	use crate::util::config::get_config;
	use crate::util::time::timestamp_minute;

	#[cfg_attr(debug_assertions, derive(Debug))]
	#[derive(Serialize, Deserialize, Default)]
	pub struct User {
		#[serde(skip)]
		_mod: ModificationTracker,
		id: i32,
		wrong_pass: i32,
		next_attempt: u64,
		name: String,
		username: String,
		password: String,
		permissions: HashSet<String>,
	}
	mod_field! {User._mod}

	#[cfg_attr(debug_assertions, derive(Debug))]
	#[derive(PartialEq)]
	pub enum PasswordCheck {
		NeedReset,
		TooManyAttempt,
		Invalid,
		Valid,
	}

	impl User {
		pub async fn check_pass(&mut self, pwd: Vec<u8>) -> PasswordCheck {
			let cfg = get_config().await;
			let max_retry = cfg.security.max_login_retry;
			let cooldown = cfg.security.login_cool_down;
			if self.password.is_empty() {
				return PasswordCheck::NeedReset;
			}
			if max_retry != -1 && self.wrong_pass >= max_retry {
				if self.next_attempt > timestamp_minute() {
					return PasswordCheck::TooManyAttempt;
				} else {
					// reset after `next_attempt` is reached
					self.wrong_pass = 0;
					self.log_modify_static("wrong_pass");
				}
			}
			drop(cfg);
			let pass = self.password.clone();
			let valid = tokio_rayon::spawn(move || {
				argon2::verify_encoded(&pass, &pwd)
			}).await.unwrap_or_default();
			if valid {
				self.wrong_pass = 0;
				self.log_modify_static("wrong_pass");
				PasswordCheck::Valid
			} else {
				self.wrong_pass += 1;
				self.log_modify_static("wrong_pass");
				if self.wrong_pass >= max_retry {
					self.next_attempt = timestamp_minute() + cooldown;
					self.log_modify_static("next_attempt");
				}
				PasswordCheck::Invalid
			}
		}

		pub async fn set_pass(&mut self, pass: impl AsRef<[u8]>) -> anyhow::Result<()> {
			let pass = pass.as_ref().to_vec();

			let hash = tokio_rayon::spawn(move || {
				let mut salt = vec![0u8; 128];
				let mut rng = rand::thread_rng();
				rng.fill_bytes(&mut salt);
				argon2::hash_encoded(pass.as_ref(), &salt, &Config::default())
			}).await?;
			self.password = hash;
			self.log_modify_static("password");
			Ok(())
		}

		pub fn has_permission(&self, permission: &str) -> bool {
			self.permissions.contains(permission)
		}

		pub fn add_permission(&mut self, permission: &str) -> bool {
			if self.permissions.insert(permission.to_owned()) {
				self.log_modify_static("permissions");
				true
			} else {
				false
			}
		}

		pub fn remove_permission(&mut self, permission: &str) -> bool {
			if self.permissions.remove(permission) {
				self.log_modify_static("permissions");
				true
			} else {
				false
			}
		}
	}

	#[cfg(test)]
	mod test {
		use crate::util::config::get_config;
		use crate::util::time::timestamp_minute;
		use crate::web::authentication::{PasswordCheck, User};

		#[test]
		fn test_user_password() {
			futures::executor::block_on(async {
				let mut user = User::default();

				user.set_pass("halo").await.expect("set password");
				assert_eq!(user.check_pass(b"halo".to_vec()).await, PasswordCheck::Valid);
				assert_eq!(user.check_pass(b"halow".to_vec()).await, PasswordCheck::Invalid);
				assert_ne!(user.wrong_pass, 0);

				let cfg = get_config().await;
				let max_retry = cfg.security.max_login_retry;
				let next_retry = cfg.security.login_cool_down;
				let now = timestamp_minute();
				drop(cfg);
				for _ in 1..max_retry {
					let res = user.check_pass(b"halow".to_vec()).await;
					assert_eq!(res, PasswordCheck::Invalid);
				}
				assert_eq!(user.check_pass(b"halow".to_vec()).await, PasswordCheck::TooManyAttempt);
				assert!(user.next_attempt >= (now + next_retry))
			})
		}
	}
}