use crate::{
	paths::binst_package_bin_dir,
	repo::{create_bin_symlink, create_install_toml, get_version_part, make_bin_temp_dir},
	utils::{get_toml_value_as_string, safer_remove_dir},
};
use libflate::gzip::Decoder;
use reqwest;
use semver::Version;
use std::{
	fs::{copy, create_dir_all, read_to_string, remove_file, File},
	io::{BufReader, Cursor, Read, Write},
	path::{Path, PathBuf},
};
use tar::Archive;
use toml::Value;

use super::{aws_provider::build_new_aws_bucket_client, BinRepo, BinRepoError, RepoInfo, S3Info};

const LATEST_TOML: &str = "latest.toml";

// repo install method(s)
impl BinRepo {
	pub async fn install(&self, stream: String) -> Result<(), BinRepoError> {
		// create the tempdir
		let tmp_dir = make_bin_temp_dir(&self.bin_name)?;

		//// download the package tar files to the folder
		let (download_url, version, tmp_gz) = match &self.install_repo {
			RepoInfo::Local(local_repo_origin) => self.download_from_local(local_repo_origin, &tmp_dir, &stream).await?,
			RepoInfo::S3(s3_info) => self.download_from_s3(s3_info, &tmp_dir, &stream).await?,
			RepoInfo::Http(base_url) => self.download_from_http(base_url, &tmp_dir, &stream).await?,
		};

		//// copy the gz file
		let package_dir = binst_package_bin_dir(&self.bin_name, &version)?;
		let gz_path = package_dir.join(format!("{}.tar.gz", self.bin_name));
		copy(&tmp_gz, &gz_path)?;

		//// unpack
		let unpacked_dir = package_dir.join("unpacked");
		if !unpacked_dir.is_dir() {
			create_dir_all(&unpacked_dir)?;
		}
		// decode gz file
		let gz_file = File::open(&gz_path)?;
		let gz_reader = BufReader::new(&gz_file);
		let mut dec = Decoder::new(gz_reader)?;
		let mut buf = Vec::new(); // TODO might want ot use a streamed file buffer
		dec.read_to_end(&mut buf)?;
		// save and expand the tar file
		let tar_path = package_dir.join(format!("{}.tar", self.bin_name));
		let mut tar_file = File::create(&tar_path)?;
		tar_file.write_all(&buf)?;
		let tar_file = File::open(&tar_path)?;
		let mut tar_archive = Archive::new(tar_file);
		tar_archive.unpack(&unpacked_dir)?;
		// remove the tar file
		remove_file(&tar_path)?;

		// create the install.toml
		create_install_toml(&package_dir, &self.install_repo.url(), &stream, &version)?;

		//// create the symlink
		let unpacked_bin = unpacked_dir.join(&self.bin_name);
		let bin_symlink_path = create_bin_symlink(&self.bin_name, &unpacked_bin)?;

		// print info
		println!(
			"Install Complete - package: {} - version: {}
  Downloaded from:  {}
  Downloaded   to:  {}
  Unpacked     at:  {}
  Symlinked    at:  {}",
			self.bin_name,
			version,
			download_url,
			gz_path.to_string_lossy(),
			unpacked_dir.to_string_lossy(),
			bin_symlink_path.to_string_lossy()
		);

		safer_remove_dir(&tmp_dir)?;

		Ok(())
	}

	pub async fn get_origin_latest_toml_content(&self, stream: &str) -> Result<String, BinRepoError> {
		let base_uri = self.origin_bin_target_uri(stream);
		// TODO - make sure it is the publish repo we want here.
		let content = match &self.install_repo {
			RepoInfo::Local(local_repo_origin) => {
				let origin_target_dir = Path::new(local_repo_origin).join(base_uri);
				let origin_latest_path = origin_target_dir.join(LATEST_TOML);
				if !origin_latest_path.is_file() {
					return Err(BinRepoError::OriginLatestNotFound(origin_latest_path.to_string_lossy().to_string()));
				}
				read_to_string(&origin_latest_path)?
			}
			RepoInfo::S3(s3_info) => {
				let S3Info {
					base,
					profile,
					bucket: bucket_name,
					..
				} = s3_info;
				let bucket = build_new_aws_bucket_client(&bucket_name, &profile).await?;
				let base_key = format!("{}/{}", base, base_uri);
				let latest_key = &format!("{}/{}", base_key, LATEST_TOML);
				let (data, _) = bucket.get_object(latest_key).await?;
				let data = std::str::from_utf8(&data).or_else(|_| Err(BinRepoError::InvalidInfoToml(latest_key.to_owned())))?;

				data.to_string()
			}
			RepoInfo::Http(base_url) => get_origin_latest_toml_content_from_base_url(&base_url, &base_uri).await?,
		};

		Ok(content)
	}

	pub async fn get_origin_latest_version(&self, stream: &str) -> Result<Version, BinRepoError> {
		let content = self.get_origin_latest_toml_content(stream).await?;
		let toml: Value = toml::from_str(&content)?;
		let version = get_toml_value_as_string(&toml, &["latest", "version"])?;
		let version = match Version::parse(&version) {
			Ok(version) => version,
			Err(_) => return Err(BinRepoError::InvalidVersionFromOrigin),
		};
		Ok(version)
	}
}

async fn get_origin_latest_toml_content_from_base_url(base_url: &str, base_uri: &str) -> Result<String, BinRepoError> {
	let latest_url = &format!("{}/{}/{}", base_url, base_uri, LATEST_TOML);
	let resp = reqwest::get(latest_url).await?;

	let data = resp.text().await?;
	Ok(data)
}

// download from http
impl BinRepo {
	async fn download_from_http(&self, http_base: &str, tmp_dir: &PathBuf, stream: &str) -> Result<(String, Version, PathBuf), BinRepoError> {
		let http_base = format!("{}/{}", http_base, self.origin_bin_target_uri(stream));
		// download the info.toml
		let version = self.get_origin_latest_version(stream).await?;

		// download the gz
		let gz_name = format!("{}.tar.gz", self.bin_name);
		let gz_url = format!("{}/{}/{}", http_base, get_version_part(&version), gz_name);
		let resp = reqwest::get(&gz_url).await?;
		let gz_tmp_path = tmp_dir.join(gz_name);
		let mut gz_file = File::create(&gz_tmp_path)?;
		let mut content = Cursor::new(resp.bytes().await?);
		std::io::copy(&mut content, &mut gz_file)?;

		Ok((gz_url, version, gz_tmp_path))
	}
}

// download from s3
impl BinRepo {
	async fn download_from_s3(&self, s3_info: &S3Info, tmp_dir: &PathBuf, stream: &str) -> Result<(String, Version, PathBuf), BinRepoError> {
		let S3Info {
			base,
			profile,
			bucket: bucket_name,
			..
		} = s3_info;

		//// bucket client
		let bucket = build_new_aws_bucket_client(bucket_name, profile).await?;
		let base_key = format!("{}/{}", base, self.origin_bin_target_uri(stream));

		//// download orignal latest version
		let version = self.get_origin_latest_version(stream).await?;

		// e.g., ...repo/bin_name/target/v0.1.2
		let origin_version_key = format!("{}/{}", base_key, get_version_part(&version));

		//// download the gz file
		let gz_name = format!("{}.tar.gz", self.bin_name);
		let gz_key = format!("{}/{}", origin_version_key, gz_name);
		let (data, _) = bucket.get_object(&gz_key).await?;
		let gz_tmp_path = tmp_dir.join(gz_name);
		let mut gz_file = File::create(&gz_tmp_path)?;
		gz_file.write_all(&data)?;

		let download_url = format!("S3://{}/{}", bucket_name, gz_key);

		Ok((download_url, version, gz_tmp_path))
	}
}

// download from local
impl BinRepo {
	async fn download_from_local(
		&self,
		local_repo_origin: &str,
		tmp_dir: &PathBuf,
		stream: &str,
	) -> Result<(String, Version, PathBuf), BinRepoError> {
		// base orign path the e.g., ..repo/bin_name/target/stream
		let base_uri = self.origin_bin_target_uri(stream);
		let origin_target_dir = Path::new(local_repo_origin).join(base_uri);

		// read the version file
		let version = self.get_origin_latest_version(stream).await?;

		// e.g., ...repo/bin_name/target/main/v0.1.2/
		let origin_dir = origin_target_dir.join(format!("{}", get_version_part(&version)));

		// check origin tar file
		let origin_gz = origin_dir.join(format!("{}.tar.gz", self.bin_name));
		if !origin_gz.is_file() {
			return Err(BinRepoError::OriginTarGzNotFound(origin_gz.to_string_lossy().to_string()));
		}

		let tmp_gz = tmp_dir.join(format!("{}.tar.gz", self.bin_name));
		copy(&origin_gz, &tmp_gz)?;

		let download_path = origin_gz.to_string_lossy().to_string();
		Ok((download_path, version, tmp_gz))
	}
}
