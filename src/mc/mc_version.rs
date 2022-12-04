#[allow(clippy::needless_lifetimes)] // sometime mc_version need static lifetime
pub fn java_for<'a>(mc_version: &'a str) -> Option<&'static JavaVersionForMc> {
	// mc version eg. 1.19 = 19
	let version = mc_version.split('.').nth(1)?.parse::<u8>().ok()?;
	if version <= 12 {
		Some(MC12)
	} else if version <= 16 {
		Some(MC16)
	} else {
		Some(MC18)
	}
}

static MC12: &JavaVersionForMc = &JavaVersionForMc {
	supported: &[8, 11, 17],
	recommended: 8,
};

static MC16: &JavaVersionForMc = &JavaVersionForMc {
	supported: &[8, 11, 17],
	recommended: 11,
};

static MC18: &JavaVersionForMc = &JavaVersionForMc {
	supported: &[17],
	recommended: 17,
};

/// Supported java version for minecraft  
/// eg. 8 = java8; 17 for java17
pub struct JavaVersionForMc {
	pub supported: &'static [u8],
	pub recommended: u8,
}