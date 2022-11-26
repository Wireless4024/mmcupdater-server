use std::io::stdout;

use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init() -> WorkerGuard {

	// below code copied from  `tracing_subscriber::fmt::init();`
	let (handler, _guard) = tracing_appender::non_blocking(stdout());
	let builder = tracing_subscriber::fmt::fmt()
		.compact()
		.with_writer(handler)
		.with_max_level(LevelFilter::TRACE);
	let subscriber = builder.finish();
	let subscriber = {
		use tracing_subscriber::{layer::SubscriberExt};
		use std::{env, str::FromStr};
		let targets = match env::var("RUST_LOG") {
			Ok(var) => Targets::from_str(&var).unwrap_or_default(),
			Err(_) => { Targets::new().with_default(LevelFilter::INFO) }
		};
		subscriber.with(targets)
	};
	subscriber.init();
	_guard
}