#[cfg(target_arch = "x86_64")]
pub static ARCH: &str = "amd64";
#[cfg(target_arch = "aarch64")]
pub static ARCH: &str = "aarch64";


#[cfg(target_os = "linux")]
pub static OS: &str = "linux";
#[cfg(target_os = "windows")]
pub static OS: &str = "windows";
#[cfg(target_os = "macos")]
pub static OS: &str = "darwin";

// this file will cause error on unsupported platform xD