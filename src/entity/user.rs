use std::cmp::min;
use std::ops::Deref;

use argon2::Config;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::Sqlite;

use derive::{filter_serialize, ValueAccess, ValueUpdate};

use crate::db::cache::DbCache;
use crate::db::TableMetadata;
use crate::mod_field;
use crate::util::config::get_config;
use crate::util::modification::ModificationTracker;
use crate::util::time::timestamp_minute;
use crate::web::sign_jwt;

#[derive(Serialize, Deserialize, Default, ValueAccess, ValueUpdate, Debug)]
#[filter_serialize(name, username, permissions)]
pub struct User {
	#[serde(skip)]
	_mod: ModificationTracker,
	id: i64,
	wrong_pass: i32,
	next_attempt: u64,
	pub name: String,
	pub username: String,
	password: String,
	permissions: String,
}
mod_field! {User._mod}

impl Clone for User {
	fn clone(&self) -> Self {
		Self {
			_mod: ModificationTracker::default(),
			id: self.id,
			wrong_pass: self.wrong_pass,
			next_attempt: self.next_attempt,
			name: self.name.clone(),
			username: self.username.clone(),
			password: self.password.clone(),
			permissions: self.permissions.clone(),
		}
	}
}

impl TableMetadata<Sqlite> for User {
	fn pk(&self) -> i64 { self.id }

	fn build_cache() -> DbCache<Self> {
		DbCache::new(32)
	}

	fn tb_name() -> &'static str { "User" }
}

#[derive(PartialEq, Debug)]
pub enum PasswordSignResult {
	NeedReset,
	TooManyAttempt(u64),
	Invalid,
	/// It will return empty string if not requested
	Valid(String),
}

impl User {
	pub async fn check_pass_and_sign(&mut self, pwd: Vec<u8>) -> PasswordSignResult {
		match self.check_pass(pwd).await {
			PasswordSignResult::Valid(_) => {
				PasswordSignResult::Valid(sign_jwt(self.id).await.expect("sign jwt"))
			}
			r => r
		}
	}

	pub async fn check_pass(&mut self, pwd: Vec<u8>) -> PasswordSignResult {
		let cfg = get_config().await;
		let max_retry = cfg.security.max_login_retry;
		let cooldown = cfg.security.login_cool_down;
		if self.password.is_empty() {
			return PasswordSignResult::NeedReset;
		}
		if max_retry != -1 && self.wrong_pass >= max_retry {
			if self.next_attempt > timestamp_minute() {
				return PasswordSignResult::TooManyAttempt(self.next_attempt);
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
			PasswordSignResult::Valid(String::new())
		} else {
			self.wrong_pass += 1;
			self.log_modify_static("wrong_pass");
			if self.wrong_pass >= max_retry {
				self.next_attempt = timestamp_minute() + cooldown;
				self.log_modify_static("next_attempt");
			}
			PasswordSignResult::Invalid
		}
	}

	pub async fn set_pass(&mut self, pass: impl AsRef<[u8]>) -> anyhow::Result<()> {
		let pass = pass.as_ref().to_vec();

		let hash = tokio_rayon::spawn(move || {
			let mut salt = vec![0u8; min(pass.len() * 16, 256)];
			let mut rng = rand::thread_rng();
			rng.fill_bytes(&mut salt);
			argon2::hash_encoded(pass.as_ref(), &salt, &Config::default())
		}).await?;
		self.password = hash;
		self.log_modify_static("password");
		Ok(())
	}

	pub fn has_permission(&self, permission: &str) -> bool {
		self.permissions == "*"
			|| self.permissions.split(',').any(|it| it == permission)
	}

	pub fn add_permission(&mut self, permission: &str) -> bool {
		if !self.has_permission(permission) {
			self.permissions.reserve(permission.len() + 1);
			if !self.permissions.is_empty() {
				self.permissions.push(',');
			}
			self.permissions.push_str(permission);
			self.log_modify_static("permissions");
			true
		} else {
			false
		}
	}

	pub fn remove_permission(&mut self, permission: &str) -> bool {
		if self.has_permission(permission) {
			let perms = self.permissions.split(',').filter(|it| *it != permission).collect::<Vec<_>>();
			let len = perms.iter().fold(0, |it, s| it + s.len());
			let mut out = String::with_capacity(len);
			for p in perms {
				out.push_str(p);
			}
			self.permissions = out;
			self.log_modify_static("permissions");
			true
		} else {
			false
		}
	}
}

#[cfg(test)]
mod test {
	use crate::entity::user::{PasswordSignResult, User};
	use crate::util::config::get_config;
	use crate::util::time::timestamp_minute;

	#[test]
	fn test_user_password() {
		futures::executor::block_on(async {
			let mut user = User::default();

			user.set_pass("halo").await.expect("set password");
			assert!(matches!(user.check_pass(b"halo".to_vec()).await, PasswordSignResult::Valid(_)));
			assert_eq!(user.check_pass(b"halow".to_vec()).await, PasswordSignResult::Invalid);
			assert_ne!(user.wrong_pass, 0);

			let cfg = get_config().await;
			let max_retry = cfg.security.max_login_retry;
			let next_retry = cfg.security.login_cool_down;
			let now = timestamp_minute();
			drop(cfg);
			for _ in 1..max_retry {
				let res = user.check_pass(b"halow".to_vec()).await;
				assert_eq!(res, PasswordSignResult::Invalid);
			}
			assert!(matches!(user.check_pass(b"halow".to_vec()).await, PasswordSignResult::TooManyAttempt(_)));
			assert!(user.next_attempt >= (now + next_retry))
		})
	}
}