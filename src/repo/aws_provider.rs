///////
// Utilities for Aws S3 resources

use super::{
	BinRepoError, BINST_REPO_BUCKET, ENV_BINST_REPO_AWS_KEY_ID, ENV_BINST_REPO_AWS_KEY_SECRET,
	ENV_BINST_REPO_AWS_REGION,
};
use dirs::home_dir;
use regex::Regex;
use s3::{creds::Credentials, Bucket, Region};
use std::collections::HashMap;
use std::env::{self, VarError};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;

const AWS_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const AWS_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const AWS_DEFAULT_REGION: &str = "AWS_DEFAULT_REGION";

#[derive(Debug)]
struct AwsCred {
	id: String,
	secret: String,
	region: String,
}

pub async fn build_new_aws_bucket_client(bucket_name: &str, profile: &Option<String>) -> Result<Bucket, BinRepoError> {
	let mut aws_cred = if bucket_name == BINST_REPO_BUCKET {
		extract_aws_cred_from_env(
			ENV_BINST_REPO_AWS_KEY_ID,
			ENV_BINST_REPO_AWS_KEY_SECRET,
			ENV_BINST_REPO_AWS_REGION,
		)
	} else {
		None
	};

	// if still none, try to get it from the profile or the default aws environment
	if aws_cred.is_none() {
		aws_cred = match profile {
			Some(profile) => extract_aws_cred_from_profile(profile).ok(),
			None => extract_aws_cred_from_env(AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_DEFAULT_REGION),
		};
	}

	let aws_cred = match aws_cred {
		Some(aws_cred) => aws_cred,
		None => return Err(BinRepoError::S3CredMissingInEnvOrProfile),
	};

	let credentials = Credentials::new(Some(&aws_cred.id), Some(&aws_cred.secret), None, None, None).unwrap();
	let region = Region::from_str(&aws_cred.region)?;
	let bucket = Bucket::new(bucket_name, region, credentials)?;

	//// Check bucket exist
	let (_, code) = bucket.head_object("/").await?;
	if code == 404 {
		let info = match profile {
			Some(profile) => format!("with profile {}", profile),
			None => "from environment variables".to_owned(),
		};
		return Err(BinRepoError::RepoS3BucketNotAccessible(bucket_name.to_owned(), info));
	};

	Ok(bucket)
}

fn extract_aws_cred_from_env(aws_key_id: &str, aws_key_secret: &str, aws_region: &str) -> Option<AwsCred> {
	// Note: style experimentation
	let env = || {
		let id = env::var(aws_key_id)?;
		let secret = env::var(aws_key_secret)?;
		let region = env::var(aws_region)?;
		Ok::<AwsCred, VarError>(AwsCred { id, secret, region })
	};

	match env() {
		Ok(cred) => Some(cred),
		Err(_) => None,
	}
}

fn extract_aws_cred_from_profile(profile: &str) -> Result<AwsCred, BinRepoError> {
	let region = extract_region_from_aws_config(profile)?;
	let id_secret = extract_id_secret_from_aws_config(profile)?;

	if let (Some(region), Some((id, secret))) = (region, id_secret) {
		Ok(AwsCred { region, id, secret })
	} else {
		Err(BinRepoError::S3CredMissingProfile(profile.to_owned()))
	}
}

fn extract_id_secret_from_aws_config(profile: &str) -> Result<Option<(String, String)>, std::io::Error> {
	// read the content
	let content = read_aws_credentials()?;
	let data = parse_aws_regex_block(&format!(r"\[{}\][\r\n]+([^\[]+)", profile), &content);
	let id = data.get("aws_access_key_id").map(|v| v.to_owned());
	let secret = data.get("aws_secret_access_key").map(|v| v.to_owned());

	if let (Some(id), Some(secret)) = (id, secret) {
		Ok(Some((id, secret)))
	} else {
		Ok(None)
	}
}

fn extract_region_from_aws_config(profile: &str) -> Result<Option<String>, std::io::Error> {
	// read the content
	let content = read_aws_config()?;
	let data = parse_aws_regex_block(&format!(r"\[profile\W{}\][\r\n]+([^\[]+)", profile), &content);
	let region = data.get("region").map(|v| v.to_owned());

	Ok(region)
}

fn parse_aws_regex_block(rgx_str: &str, content: &str) -> HashMap<String, String> {
	let re = Regex::new(rgx_str).unwrap();
	let caps = re.captures(content).unwrap();
	let block = caps.get(1).map_or("", |m| m.as_str()).to_owned();
	parse_aws_block(&block)
}

fn parse_aws_block(block: &str) -> HashMap<String, String> {
	let mut data = HashMap::new();

	for line in block.lines() {
		let mut parts = line.splitn(2, '=').map(|s| s.trim());
		let name = parts.next().map(|s| s.trim().to_owned());
		let value = parts.next().map(|s| s.trim().to_owned());
		if let (Some(name), Some(value)) = (name, value) {
			data.insert(name, value);
		}
	}

	data
}

fn read_aws_credentials() -> Result<String, std::io::Error> {
	let aws_config = home_dir().expect("no home").join("./.aws/credentials");
	let mut f = File::open(aws_config)?;
	let mut buffer = String::new();
	f.read_to_string(&mut buffer)?;
	Ok(buffer)
}

fn read_aws_config() -> Result<String, std::io::Error> {
	let aws_config = home_dir().expect("no home").join("./.aws/config");
	let mut f = File::open(aws_config)?;
	let mut buffer = String::new();
	f.read_to_string(&mut buffer)?;
	Ok(buffer)
}

//// Test assuming some local setup
#[cfg(test)]
mod tests_jc_only {
	use super::*;

	// #[test]
	fn _cred_from_profile() {
		assert!(extract_aws_cred_from_profile("jc-user").is_ok())
	}
}
