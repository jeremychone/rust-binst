use crate::repo::Result;
use crate::repo::{S3Info, BINST_REPO_AWS_PROFILE, BINST_REPO_BUCKET, BINST_REPO_URL};
use crate::utils::clean_path;

#[derive(Debug)]
pub enum RepoInfo {
	// local path dir
	Local(String),
	// S3, only support via profile for now
	S3(S3Info),
	// http/https, only for install
	Http(String),
}

impl RepoInfo {
	pub fn url(&self) -> &str {
		match self {
			RepoInfo::Local(url) => url,
			RepoInfo::S3(s3_info) => &s3_info.url,
			RepoInfo::Http(url) => url,
		}
	}
}

/// Builders
impl RepoInfo {
	pub fn binst_publish_repo() -> RepoInfo {
		let s3_info = S3Info {
			url: format!("s3://{}", BINST_REPO_BUCKET),
			bucket: BINST_REPO_BUCKET.to_string(),
			base: "".to_string(),
			profile: Some(BINST_REPO_AWS_PROFILE.to_string()),
		};
		RepoInfo::S3(s3_info)
	}

	pub fn binst_install_repo() -> RepoInfo {
		RepoInfo::Http(clean_path(BINST_REPO_URL))
	}

	pub fn from_repo_string(repo: &str, profile: Option<&str>) -> Result<RepoInfo> {
		let repo_info = if repo.starts_with("s3://") {
			RepoInfo::S3(S3Info::from_s3_url(repo, profile)?)
		} else if repo.starts_with("http://") || repo.starts_with("https://") {
			RepoInfo::Http(clean_path(repo))
		} else {
			RepoInfo::Local(clean_path(repo))
		};

		Ok(repo_info)
	}
}
