//! AWS S3 (Official lib) wrapper

use crate::repo::{Error, Result};
use aws_config::profile::profile_file::ProfileFiles;
use aws_config::profile::Profile;
use aws_sdk_s3::config::Builder;
use aws_sdk_s3::{Client, Credentials, Region};
use aws_types::os_shim_internal::{Env, Fs};
use std::env;

struct CredEnv {
	key_id: &'static str,
	key_secret: &'static str,
	region: &'static str,
	endpoint: &'static str,
}

const BINST_CRED_ENV: CredEnv = CredEnv {
	key_id: "BINST_REPO_AWS_KEY_ID",
	key_secret: "BINST_REPO_AWS_KEY_SECRET",
	region: "BINST_REPO_AWS_REGION",
	endpoint: "BINST_REPO_AWS_ENDPOINT",
};

const AWS_CRED_ENV: CredEnv = CredEnv {
	key_id: "AWS_ACCESS_KEY_ID",
	key_secret: "AWS_SECRET_ACCESS_KEY",
	region: "AWS_DEFAULT_REGION",
	endpoint: "AWS_ENDPOINT",
};

pub async fn new_aws_client(profile_str: Option<String>) -> Result<Client> {
	let AwsCred {
		key_id,
		key_secret,
		region,
		endpoint,
	} = {
		// Note: using inline match for await.
		let cred = match profile_str {
			Some(profile_str) => aws_cred_from_aws_profile_configs(&profile_str).await.transpose(),
			None => None,
		};
		// Note: The Result<Option>/Transpose technic below allows to chain env resolution
		//       while preserving the initial error if present.
		cred.or_else(|| BINST_CRED_ENV.aws_cred().transpose())
			.or_else(|| AWS_CRED_ENV.aws_cred().transpose())
			.transpose()? // flip back to Error<Option<>> to be able to do ?
			.ok_or(Error::S3CredMissingInEnvOrProfile)? // if still None, return error.
	};

	let cred = Credentials::new(key_id, key_secret, None, None, "loaded-from-config-or-env");
	let mut builder = Builder::new().credentials_provider(cred);

	if let Some(endpoint) = endpoint {
		// WORKAROUND - Right now the aws-sdk throw a NoRegion on .send if not region even if we have a endpoint
		builder = builder.endpoint_url(endpoint).region(Region::new("endpoint-region"));
	}

	if let Some(region) = region {
		builder = builder.region(Region::new(region));
	}

	let config = builder.build();
	let client = Client::from_conf(config);
	Ok(client)
}

#[derive(Debug)]
struct AwsCred {
	key_id: String,
	key_secret: String,
	region: Option<String>,
	endpoint: Option<String>,
}

impl CredEnv {
	/// Look if there is the corresponding AWS credentials
	/// - If none is found, then, return Ok(None). (normal)
	/// - If partial, then, return error.
	/// - If no region nor endpoint, return error.
	fn aws_cred(&self) -> Result<Option<AwsCred>> {
		let key_id = get_env(self.key_id).ok();
		let key_secret = get_env(self.key_secret).ok();
		let region = get_env(self.region).ok();
		let endpoint = get_env(self.endpoint).ok();

		match (key_id, key_secret) {
			// both are missing, then this env sets is not set, so None
			(None, None) => Ok(None),
			// both sets, so all good
			(Some(key_id), Some(key_secret)) => {
				// TODO: Check that we have at least region or endpoint (both cannot be None)
				if let (None, None) = (&region, &endpoint) {
					return Err(Error::S3CredMustHaveRegionOrEndpoint);
				}
				Ok(Some(AwsCred {
					key_id,
					key_secret,
					region,
					endpoint,
				}))
			}
			// if only one is set, then error.
			_ => Err(Error::S3CredMissingInEnvOrProfile),
		}
	}
}

async fn aws_cred_from_aws_profile_configs(profile_str: &str) -> Result<Option<AwsCred>> {
	let (fs, ev) = (Fs::real(), Env::default());
	let profiles = aws_config::profile::load(&fs, &ev, &ProfileFiles::default(), None).await;

	if let Ok(profiles) = profiles {
		if let Some(profile) = profiles.get_profile(profile_str) {
			let key_id = get_profile_value(profile, "aws_access_key_id")?;
			let key_secret = get_profile_value(profile, "aws_secret_access_key")?;
			let region = get_profile_value(profile, "region").ok();
			// Note: endpoint is not really standard for the .aws/config, but here in case present.
			let endpoint = get_profile_value(profile, "endpoint").ok();

			// TODO: Needs to error if region is missing

			return Ok(Some(AwsCred {
				key_id,
				key_secret,
				region,
				endpoint, // because aws configs only
			}));
		}
	}

	Ok(None)
}

// region:    --- Cred Private Utils
fn get_env(name: &str) -> Result<String> {
	match env::var(name) {
		Ok(v) => Ok(v),
		Err(_) => Err(Error::NoCredentialEnv(name.to_string())),
	}
}
fn get_profile_value(profile: &Profile, key: &str) -> Result<String> {
	match profile.get(key) {
		Some(value) => Ok(value.to_string()),
		None => Err(Error::NoCredentialConfig(key.to_string())),
	}
}
// endregion: --- Cred Private Utils
