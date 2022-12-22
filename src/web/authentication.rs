use futures::join;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::Private;
use openssl::rsa::Rsa;
use rand::RngCore;
use tokio::fs::metadata;
use tracing::{debug, info};

use jwt::{DECODE_KEY, DEFAULT_PRI, DEFAULT_PUB, ENCODE_KEY};
pub use jwt::{Authorization, sign_jwt};

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
				.unwrap_or(DEFAULT_PRI)).await?;
		let dec_key = tokio::fs::read(
			Some(&jwt.dec_key)
				.and_then(|it| if it.is_empty() { None } else { Some(it.as_str()) })
				.unwrap_or(DEFAULT_PUB)).await?;

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
	use jsonwebtoken::{decode, DecodingKey, encode, EncodingKey, Header, Validation};
	use serde::{Deserialize, Serialize};
	use tokio::sync::OnceCell;

	use crate::util::config::get_config;
	use crate::util::errors::ErrorWrapper;
	use crate::util::time::timestamp_minute;

	pub(crate) static ENCODE_KEY: OnceCell<EncodingKey> = OnceCell::const_new();
	pub(crate) static DECODE_KEY: OnceCell<DecodingKey> = OnceCell::const_new();

	pub(crate) static DEFAULT_PRI: &str = "jwt.key";
	pub(crate) static DEFAULT_PUB: &str = "jwt.pub";

	pub async fn sign_jwt(user_id: i64) -> anyhow::Result<String> {
		let (jwt_algo, exp_mins) = {
			let jwt = &get_config().await.http.jwt;
			(jwt.algo, jwt.valid_time)
		};
		Ok(tokio_rayon::spawn(move || {
			let exp = (timestamp_minute() + exp_mins) * 60;
			encode(&Header::new(jwt_algo), &JwtContent { id: user_id, exp }, ENCODE_KEY.get().expect("Jwt encode key"))
		}).await?)
	}

	#[derive(Serialize, Deserialize)]
	pub struct JwtContent {
		pub id: i64,
		pub exp: u64,
	}

	pub struct Authorization(pub JwtContent);

	impl Authorization {
		pub async fn refresh(&self) -> String {
			sign_jwt(self.0.id).await.expect("Sign jwt")
		}
	}

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
			} else {
				let cookies = headers.get_all("cookie");
				for v in cookies.into_iter() {
					if v.as_bytes().starts_with(b"authorization") {
						let s = String::from_utf8_lossy(v.as_bytes());
						if let Ok(c) = cookie::Cookie::parse(s) {
							if c.name() == "authorization" {
								let jwt_algo = {
									get_config().await.http.jwt.algo
								};
								match decode::<JwtContent>(
									c.value(),
									DECODE_KEY.get().unwrap(),
									&Validation::new(jwt_algo)) {
									Ok(claim) => {
										return Ok(Authorization(claim.claims));
									}
									Err(_) => {
										// does nothing for now just request to re-login
										// Err::<(), _>(err).expect("");
									}
								};
								break;
							}
						};
					}
				}
			};

			Err(ErrorWrapper::custom(StatusCode::UNAUTHORIZED, "Unauthorized"))
		}
	}
}