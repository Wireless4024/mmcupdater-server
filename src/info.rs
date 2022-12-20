use std::fs::File;
use std::io::{BufRead, BufReader};

use serde::Serialize;
use sys_info::{LoadAvg, MemInfo};
use tokio::task::{JoinHandle, spawn_blocking};

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
	os: String,
	arch: &'static str,
}

impl Default for DetailedInfo {
	fn default() -> Self {
		Self {
			_info: get_sys_info().unwrap_or_default(),
			os: sys_info::os_release()
				.map(|it| format!("{} {it}", std::env::consts::OS))
				.unwrap_or_else(|_| std::env::consts::OS.to_string()),
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
	mem_shm: u64,
	mem_used: u64,
	swap_total: u64,
	swap_free: u64,
	load_1: f64,
	load_5: f64,
	load_15: f64,
}

#[inline]
fn sys_info_async() -> JoinHandle<Option<SysInfo>> {
	spawn_blocking(get_sys_info)
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
	let mem_shm = {
		(|| {
			let f = File::open("/proc/meminfo").ok()?;
			let mut reader = BufReader::new(f);
			let mut buf = String::new();
			loop {
				let len = reader.read_line(&mut buf).ok()?;
				if len == 0 { break None; }
				if buf.starts_with("Shmem") {
					let mut split = buf.split_ascii_whitespace();
					split.next();
					break split.next().and_then(|it| it.parse::<u64>().ok());
				}
				buf.clear();
			}
		})().unwrap_or_default()
	};
	let mem_used = mem_total - (mem_free + mem_cache);
	Some(SysInfo {
		hostname,
		cpus,
		cpu_clock,
		mem_total,
		mem_free,
		mem_avail,
		mem_buff,
		mem_cache,
		mem_shm,
		mem_used,
		swap_total,
		swap_free,
		load_1,
		load_5,
		load_15,
	})
}