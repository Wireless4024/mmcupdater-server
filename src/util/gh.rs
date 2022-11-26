use serde::Deserialize;

use crate::util::errors::ErrorWrapper;
use crate::util::http::new_client;

/// fetch latest release from github repo  
/// repo: <OWNER>/<REPO>
pub async fn get_gh_latest_release(repo: &str) -> Result<Option<GhRelease>, ErrorWrapper> {
	let resp: Option<GhRelease> = new_client()?
		.get(format!("https://api.github.com/repos/{repo}/releases/latest"))
		.send()
		.await?
		.json()
		.await?;
	Ok(resp)
}
/// fetch latest release from github repo  
/// repo: <OWNER>/<REPO>
/// tag: Release tag from github (can be found at <REPO_URL>/tags)
pub async fn get_gh_release_from_tag(repo: &str,tag:&str) -> Result<Option<GhRelease>, ErrorWrapper> {
	let resp: Option<GhRelease> = new_client()?
		.get(format!("https://api.github.com/repos/{repo}/releases/tags/{tag}"))
		.send()
		.await?
		.json()
		.await?;
	Ok(resp)
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Deserialize)]
pub struct ReleaseAsset {
	pub url: String,
	pub id: i64,
	pub node_id: String,
	pub name: String,
	//pub label: String,
	//pub uploader: GhUser,
	pub content_type: String,
	//pub state: String,
	pub size: i64,
	//pub download_count: i64,
	//pub created_at: String,
	//pub updated_at: String,
	pub browser_download_url: String,
}
/*
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Deserialize)]
pub struct GhUser {
	pub login: String,
	pub id: i64,
	pub node_id: String,
	pub avatar_url: String,
	pub gravatar_id: String,
	pub url: String,
	pub html_url: String,
	pub followers_url: String,
	pub following_url: String,
	pub gists_url: String,
	pub starred_url: String,
	pub subscriptions_url: String,
	pub organizations_url: String,
	pub repos_url: String,
	pub events_url: String,
	pub received_events_url: String,
	#[serde(rename = "type")]
	pub r#type: String,
	pub site_admin: bool,
}*/

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Deserialize)]
pub struct GhRelease {
	pub url: String,
	pub assets_url: String,
	//pub upload_url: String,
	//pub html_url: String,
	//pub id: i64,
	//pub author: GhUser,
	//pub node_id: String,
	pub tag_name: String,
	//pub target_commitish: String,
	pub name: String,
	pub draft: bool,
	pub prerelease: bool,
	//pub created_at: String,
	//pub published_at: String,
	#[serde(default)]
	pub assets: Vec<ReleaseAsset>,
	//pub tarball_url: String,
	pub zipball_url: String,
	//pub body: String,
	//pub reactions: Option<GhReaction>,
}