use std::env::args_os;
use std::fmt::Display;
use std::future::Future;
use std::io::stdin;
use std::process::exit;

use clap::{Parser, Subcommand};
use sqlx::{Pool, Sqlite};

use user::UserCommand;

use crate::db;
use crate::db::DbWrapper;

mod user;

pub fn intercept() {
	if args_os().len() > 1 {
		let cli: Cli = Cli::parse();
		cli.cmd.handle();
		exit(0)
	}
}

fn run_async<F: Future>(fut: F) {
	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.global_queue_interval(255)
		.build()
		.unwrap();

	rt.block_on(fut);
}

fn run_async_db<F: FnOnce(DbWrapper<Sqlite, Pool<Sqlite>>) -> Fut, Fut: Future>(f: F) {
	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.global_queue_interval(255)
		.build()
		.unwrap();
	let db = rt.block_on(get_db());
	rt.block_on(f(db));
}

async fn get_db() -> DbWrapper<Sqlite, Pool<Sqlite>> {
	db::init().await.unwrap()
}


fn ask(hint: &str) -> String {
	print!("{hint} >");
	let mut s = String::new();
	stdin().read_line(&mut s).unwrap();
	s = s.trim().to_string();
	s
}

fn ask_pwd(hint: &str) -> String {
	rpassword::prompt_password(hint).unwrap_or_else(|_| ask(hint))
}

#[inline]
fn halt(msg: impl Display) -> ! {
	eprintln!("{msg}");
	exit(1)
}

#[inline]
fn halt_skipped() -> ! {
	eprintln!("Skipped..");
	exit(0)
}

#[inline]
fn success() {
	println!("Ok")
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
	#[command(subcommand)]
	cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Manage user
	User {
		#[command(subcommand)]
		cmd: UserCommand
	},
}

impl Commands {
	#[inline]
	pub fn handle(self) {
		match self {
			Commands::User { cmd } => {
				cmd.handle();
			}
		}
	}
}
