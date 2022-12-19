use clap::Subcommand;

use crate::cli::ask;
use crate::db::TableMetadata;
use crate::entity::user::{PasswordSignResult, User};
use crate::util::string::BoolExt;

use super::{ask_pwd, halt, halt_skipped, run_async_db, success};

//noinspection SpellCheckingInspection
#[derive(Subcommand)]
pub(crate) enum UserCommand {
	/// Add user
	Add { username: String },
	/// Change password
	Chpwd { username: String },
}

impl UserCommand {
	#[inline]
	pub fn handle(self) {
		match self {
			UserCommand::Add { username } => {
				Self::add(username)
			}
			UserCommand::Chpwd { username } => {
				Self::chpwd(username)
			}
		}
	}

	fn add(username: String) {
		run_async_db(|db| async move {
			let repo = db.repo::<User>();
			let mut user = User::default();
			user.username = username;
			if let Some(u) = repo.get_by(&["username"], &user).await {
				halt(format!("This username already existed!, with user id = {}", u.pk()));
			}
			let pwd = ask_pwd("Enter password, leave blank to skip");
			if !pwd.is_empty() {
				user.set_pass(pwd.as_bytes()).await.expect("Set password");
			}
			let admin = ask("admin? y or enter");
			if admin.maybe_true() {
				user.add_permission("*");
			}
			// without cache it will affect to sqlite
			repo.perform_insert_minimal(&user).await.expect("Update user");
			println!("Ok");
			db.close().await
		});
	}

	fn chpwd(username: String) {
		run_async_db(|db| async move {
			let repo = db.repo::<User>();
			let mut u = User::default();
			u.username = username;
			let Some(mut user) = repo.get_by(&["username"], &u).await else {
				halt(format!("username={} doesn't existed!", u.username));
			};
			let pwd = ask_pwd("Enter password, leave blank to skip");
			if !pwd.is_empty() {
				user.set_pass(pwd.as_bytes()).await.expect("Set password");
			} else {
				halt_skipped();
			}
			// without cache it will affect to sqlite
			repo.update(&user).await.expect("Update user");
			let mut user = repo.get_by(&["username"], &u).await.expect("User");
			match user.check_pass(pwd.into_bytes()).await {
				PasswordSignResult::Valid(_) => {
					success()
				}
				_ => {
					halt("Something wrong, password can't verified!")
				}
			}

			db.close().await
		});
	}
}