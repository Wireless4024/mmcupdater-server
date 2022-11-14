use std::future::Future;
use std::io::Result;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{debug, info, trace, warn};

use crate::instance::mc_server::MinecraftServerStatus::{RUNNING, STARTING, STOPPED};

pub struct MinecraftServer {
	pub(crate) process: Arc<Mutex<Option<Child>>>,
	pub(crate) stdin: RwLock<Option<BufWriter<ChildStdin>>>,
	pub(crate) status: Arc<RwLock<MinecraftServerStatus>>,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum MinecraftServerStatus {
	STARTING,
	RUNNING,
	CRASHED,
	STOPPED,
}

impl MinecraftServer {
	pub fn new(process: Option<Child>) -> Result<Self> {
		if let Some(mut process) = process {
			let stdout = process.stdout.take().unwrap();
			let stdin = BufWriter::new(process.stdin.take().unwrap());
			let status = Arc::new(RwLock::new(STARTING));
			let status_clone = status.clone();

			let process = Arc::new(Mutex::new(Some(process)));
			let process_clone = process.clone();

			trace!("starting server");
			let this = Self { process, stdin: RwLock::new(Some(stdin)), status };
			this.create_heartbeat(stdout, status_clone, process_clone);
			Ok(this)
		} else {
			Ok(Self {
				process: Arc::new(Mutex::new(None)),
				stdin: RwLock::new(None),
				status: Arc::new(RwLock::new(STOPPED)),
			})
		}
	}

	pub(crate) fn create_heartbeat(&self, stdout: ChildStdout, status: Arc<RwLock<MinecraftServerStatus>>, process_clone: Arc<Mutex<Option<Child>>>) {
		trace!("spawning heartbeat task");
		tokio::spawn(async move {
			let pid = {
				*status.write().await = STARTING;
				let process = process_clone.lock().await;
				process.as_ref().map(|it| it.id()).unwrap_or_default().unwrap_or_default()
			};
			trace!("starting heartbeat");
			let mut stdout = BufReader::new(stdout).lines();
			trace!("waiting for output");

			let mut early_exit = false;

			loop {
				if let Ok(res) = timeout(Duration::from_secs(30), stdout.next_line()).await {
					if let Ok(Some(line)) = res {
						if line.contains(r#"For help, type "help""#) {
							info!("found help message; server started!");
							let mut s = status.write().await;
							if *s == STOPPED {
								early_exit = true;
								break;
							}
							*s = RUNNING;
							break;
						}
					} else {
						break;
					}
				}
			}

			if !early_exit {
				debug!("reading output in background");
				loop {
					if let Ok(res) = timeout(Duration::from_secs(30), stdout.next_line()).await {
						if let Ok(Some(_)) = res {} else {
							break;
						}
						sleep(Duration::from_millis(20)).await;
					}
				}
			}

			debug!("Output stopped checking status");
			sleep(Duration::from_secs(1)).await;
			{
				if *status.read().await == STOPPED {
					return Result::<()>::Ok(());
				};
			}
			let mut process = process_clone.lock().await;
			if let Some(ref mut process) = *process {
				if let Some(id) = process.id() {
					if id != pid {
						return Result::<()>::Ok(());
					}
				}
			}
			drop(process);
			loop {
				let mut process = process_clone.lock().await;
				debug!("Waiting process to exit");
				if let Some(ref mut process) = *process {
					if let Ok(Some(estatus)) = process.try_wait() {
						let mut s = status.write().await;
						if estatus.success() {
							*s = STOPPED;
						} else {
							warn!("Server crashed!");
							*s = MinecraftServerStatus::CRASHED;
						}
						break;
					}
				} else { // process is taken by stop/kill function
					break;
				}
				drop(process);
				sleep(Duration::from_secs(2)).await;
			}
			debug!("Killing heartbeat thread..");
			Result::<()>::Ok(())
		});
	}

	pub async fn status(&self) -> MinecraftServerStatus {
		*self.status.read().await
	}

	pub async fn wait_started(&self) -> Result<()> {
		info!("waiting server to start");
		loop {
			let status = self.status().await;
			if status != STARTING {
				break;
			}
			sleep(Duration::from_secs(2)).await;
		}
		Ok(())
	}

	pub async fn input(&self, message: impl AsRef<[u8]>) -> Result<()> {
		let data = message.as_ref();
		let mut stdin = self.stdin.write().await;
		if let Some(stdin) = stdin.as_mut() {
			stdin.write_all(data).await?;
			if let Some(b'\n') = data.last() {} else {
				stdin.write(&[b'\n']).await?;
			}
			stdin.flush().await?;
		}
		drop(stdin);

		Ok(())
	}

	pub async fn say(&self, message: impl AsRef<str>) -> Result<()> {
		let data = message.as_ref();
		let mut msg = String::with_capacity(5 + data.as_bytes().len());
		msg.push_str("say ");
		msg.push_str(data);
		msg.push('\n');
		let mut stdin = self.stdin.write().await;
		if let Some(stdin) = stdin.as_mut() {
			stdin.write_all(msg.as_bytes()).await?;
			stdin.flush().await?;
		}
		drop(stdin);
		Ok(())
	}

	pub async fn restart_in_place(&self, spawn: impl FnOnce() -> Pin<Box<dyn Future<Output=Result<Child>>>>) -> Result<()> {
		if self.status().await == STARTING {
			warn!("Server is starting this restart will do nothing.");
			return Ok(());
		}
		self.shutdown_in_place().await.ok();
		let mut child = spawn().await?;
		if let Some(stdout) = child.stdout.take() {
			self.create_heartbeat(stdout, self.status.clone(), self.process.clone());
		}
		let stdin = child.stdin.take();
		*self.process.lock().await = Some(child);
		*self.stdin.write().await = stdin.map(|it| BufWriter::new(it));
		Ok(())
	}

	async fn shutdown(&self, soft: bool) -> Result<()> {
		debug!("stopping server");
		let mut sin = self.stdin.write().await;
		trace!("taking stdin");
		let stdin = sin.take();
		drop(sin);
		let status = self.status().await;

		if let (Some(mut stdin), true) = (stdin, soft) {
			if status == RUNNING || status == STARTING {
				debug!("Server is running! stop event will wait for 15 seconds");
				stdin.write_all("say Server will stop within 15 seconds\n".as_bytes()).await?;
				stdin.flush().await?;
				let mut no = String::with_capacity(8);
				use std::fmt::Write;
				for counter in (0..=14).rev() {
					sleep(Duration::from_secs(1)).await;
					no.clear();
					no.push_str("say ");
					writeln!(&mut no, "{}", counter).ok();
					stdin.write_all(no.as_bytes()).await?;
					stdin.flush().await?;
				}
			}

			stdin.write_all("stop\n".as_bytes()).await?;
			stdin.flush().await?;
			sleep(Duration::from_secs(1)).await;
		} else if soft {
			trace!("can't take stdin!");
		}

		if let Some(mut process) = self.process.lock().await.take() {
			if soft {
				if let Ok(estatus) = process.wait().await {
					let mut s = self.status.write().await;
					if estatus.success() {
						*s = STOPPED;
					} else {
						warn!("Server crashed!");
						*s = MinecraftServerStatus::CRASHED;
					}
				};
				process.kill().await?;
			} else {
				process.kill().await?;
				*self.status.write().await = STOPPED;
			}
		}
		Ok(())
	}

	pub async fn kill(&self) -> Result<()> {
		self.shutdown(false).await
	}

	pub async fn shutdown_in_place(&self) -> Result<()> {
		self.shutdown(true).await
	}

	pub async fn stop(self) -> Result<()> {
		debug!("Sending stop command to server");
		self.shutdown_in_place().await?;
		info!("Server stopped");
		Ok(())
	}
}