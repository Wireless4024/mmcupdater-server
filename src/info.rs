use serde::Serialize;
use sys_info::{LoadAvg, MemInfo};

#[derive(Serialize)]
pub struct GlobalInfo {
	version: &'static str,
	api_version: u32,
}

impl GlobalInfo {
	pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
	pub const API_VERSION: u32 = 1;
}

impl Default for GlobalInfo {
	fn default() -> Self {
		Self {
			version: GlobalInfo::VERSION,
			api_version: GlobalInfo::API_VERSION,
		}
	}
}

#[derive(Serialize)]
pub struct DetailedInfo {
	#[serde(flatten)]
	_info: SysInfo,
	os: &'static str,
	arch: &'static str,
}

impl Default for DetailedInfo {
	fn default() -> Self {
		Self {
			_info: get_sys_info().unwrap_or_default(),
			os: std::env::consts::OS,
			arch: std::env::consts::ARCH,
		}
	}
}

#[derive(Serialize, Default)]
struct SysInfo {
	hostname: String,
	cpus: u32,
	cpu_clock: u64,
	mem_total: u64,
	mem_free: u64,
	mem_avail: u64,
	mem_buff: u64,
	mem_cache: u64,
	swap_total: u64,
	swap_free: u64,
	load_1: f64,
	load_5: f64,
	load_15: f64,
}

fn get_sys_info() -> Option<SysInfo> {
	let cpus = sys_info::cpu_num().ok()?;
	let cpu_clock = sys_info::cpu_speed().ok()?;
	let MemInfo {
		total: mem_total,
		free: mem_free,
		avail: mem_avail,
		buffers: mem_buff,
		cached: mem_cache,
		swap_total,
		swap_free
	} = sys_info::mem_info().ok()?;
	let hostname = sys_info::hostname().ok()?;
	let LoadAvg { one: load_1, five: load_5, fifteen: load_15 } = sys_info::loadavg().ok()?;

	Some(SysInfo {
		hostname,
		cpus,
		cpu_clock,
		mem_total,
		mem_free,
		mem_avail,
		mem_buff,
		mem_cache,
		swap_total,
		swap_free,
		load_1,
		load_5,
		load_15,
	})
}