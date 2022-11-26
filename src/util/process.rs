use std::ffi::OsStr;
use std::io;
use std::io::ErrorKind;
use std::process::Stdio;

use bstr::ByteSlice;
use tokio::process::Command;
use tracing::trace;

pub async fn eval(cmd: impl IntoIterator<Item=impl AsRef<OsStr> + Clone>) -> io::Result<Vec<u8>> {
	let command = cmd.into_iter().collect::<Vec<_>>();
	trace!("evaluating command {:?}",command.iter().fold(Vec::new(),|mut a,b|{
		#[cfg(target_os = "linux")]
		{
			use std::os::unix::ffi::OsStrExt;
			a.extend(AsRef::<OsStr>::as_ref(b).as_bytes());
		}
		#[cfg(not(target_os = "linux"))]
		a.extend(AsRef::<OsStr>::as_ref(b).to_string_lossy().as_bytes());
		a.push(b' ');
		a
	}).as_bstr().trim().as_bstr());
	let mut iter = command.into_iter();
	let cmd = iter.next().ok_or(io::Error::new(ErrorKind::InvalidFilename, "Invalid executable"))?;
	let proc = Command::new(cmd)
		.args(iter)
		.kill_on_drop(true)
		.stdin(Stdio::null())
		.stderr(Stdio::piped())
		.stdout(Stdio::piped())
		.spawn()?;
	let output = proc.wait_with_output().await?;
	let status = output.status;
	if status.success() {
		if output.stdout.is_empty() {
			Ok(output.stderr)
		} else {
			Ok(output.stdout)
		}
	} else {
		Err(io::Error::new(ErrorKind::Other, "Failed to run command"))
	}
}