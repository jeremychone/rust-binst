use super::s3w::new_repo_bucket;
use super::{BinRepo, RepoInfo, S3Info};
use crate::paths::binst_package_bin_dir;
use crate::repo::s3w::get_full_key_and_s3_url;
use crate::repo::{create_bin_symlink, create_install_toml, get_version_part, make_bin_temp_dir};
use crate::repo::{Error, Result};
use crate::utils::{get_toml_value_as_string, safer_remove_dir};
use libflate::gzip::Decoder;
use semver::Version;
use std::fs::{copy, create_dir_all, read_to_string, remove_file, File};
use std::io::{BufReader, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use toml::Value;

const LATEST_TOML: &str = "latest.toml";

// repo install method(s)
impl BinRepo {
	pub async fn install(&self, stream: String) -> Result<()> {
		// create the tempdir
		let tmp_dir = make_bin_temp_dir(&self.bin_name)?;

		//// download the package tar files to the folder
		let (download_url, version, tmp_gz) = match &self.install_repo {
			RepoInfo::Local(local_repo_origin) => {
				self.download_from_local(local_repo_origin, &tmp_dir, &stream).await?
			}
			RepoInfo::S3(s3_info) => self.download_from_s3(s3_info, &tmp_dir, &stream).await?,
			RepoInfo::Http(base_url) => self.download_from_http(base_url, &tmp_dir, &stream).await?,
		};

		//// copy the gz file
		let package_dir = binst_package_bin_dir(&self.bin_name, &version)?;
		let gz_path = package_dir.join(format!("{}.tar.gz", self.bin_name));
		copy(tmp_gz, &gz_path)?;

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
		create_install_toml(&package_dir, self.install_repo.url(), &stream, &version)?;

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

	pub async fn get_origin_latest_toml_content(&self, stream: &str) -> Result<String> {
		let base_target_uri = self.origin_bin_target_uri(stream);

		// TODO - make sure it is the publish repo we want here.
		let content = match &self.install_repo {
			RepoInfo::Local(local_repo_origin) => {
				let origin_target_dir = Path::new(local_repo_origin).join(base_target_uri);
				let origin_latest_path = origin_target_dir.join(LATEST_TOML);
				if !origin_latest_path.is_file() {
					return Err(Error::OriginLatestNotFound(
						origin_latest_path.to_string_lossy().to_string(),
					));
				}
				read_to_string(&origin_latest_path)?
			}
			RepoInfo::S3(s3_info) => {
				let bucket = new_repo_bucket(s3_info.profile.clone()).await?;
				let key = format!("{base_target_uri}/{LATEST_TOML}");
				bucket.download_to_string(s3_info, &key).await?
			}
			RepoInfo::Http(base_url) => {
				get_origin_latest_toml_content_from_base_url(base_url, &base_target_uri).await?
			}
		};

		Ok(content)
	}

	pub async fn get_origin_latest_version(&self, stream: &str) -> Result<Version> {
		let content = self.get_origin_latest_toml_content(stream).await?;
		let toml: Value = toml::from_str(&content)?;
		let version = get_toml_value_as_string(&toml, &["latest", "version"])?;
		let version = match Version::parse(&version) {
			Ok(version) => version,
			Err(_) => return Err(Error::InvalidVersionFromOrigin),
		};
		Ok(version)
	}
}

async fn get_origin_latest_toml_content_from_base_url(base_url: &str, base_uri: &str) -> Result<String> {
	let latest_url = &format!("{}/{}/{}", base_url, base_uri, LATEST_TOML);
	let resp = reqwest::get(latest_url).await?;

	let data = resp.text().await?;
	Ok(data)
}

// download from http
impl BinRepo {
	async fn download_from_http(
		&self,
		http_base: &str,
		tmp_dir: &Path,
		stream: &str,
	) -> Result<(String, Version, PathBuf)> {
		let version = self.get_origin_latest_version(stream).await?;

		let gz_url = self.get_origin_http_url(http_base, stream, &version)?;
		let gz_name = gz_url.rsplit_once('/').unwrap().1; // We know it must have one.

		let resp = reqwest::get(&gz_url).await?;
		let gz_tmp_path = tmp_dir.join(gz_name);
		let mut gz_file = File::create(&gz_tmp_path)?;
		let mut content = Cursor::new(resp.bytes().await?);
		std::io::copy(&mut content, &mut gz_file)?;

		Ok((gz_url, version, gz_tmp_path))
	}

	pub fn get_origin_http_url(&self, http_base: &str, stream: &str, version: &Version) -> Result<String> {
		let http_base = format!("{}/{}", http_base, self.origin_bin_target_uri(stream));
		let gz_name = format!("{}.tar.gz", self.bin_name);
		let gz_url = format!("{}/{}/{}", http_base, get_version_part(version), gz_name);

		Ok(gz_url)
	}
}

// download from s3
impl BinRepo {
	async fn download_from_s3(
		&self,
		s3_info: &S3Info,
		tmp_dir: &Path,
		stream: &str,
	) -> Result<(String, Version, PathBuf)> {
		// -- download orignal latest version
		let version = self.get_origin_latest_version(stream).await?;

		let (gz_name, gz_key) = self.get_name_and_key(stream, &version);

		// -- download the gz file

		let gz_tmp_path = tmp_dir.join(gz_name);

		let bucket = new_repo_bucket(s3_info.profile.clone()).await?;
		let download_url = bucket.download_to_file(s3_info, &gz_key, &gz_tmp_path).await?;

		Ok((download_url, version, gz_tmp_path))
	}

	pub fn get_origin_s3_url(&self, s3_info: &S3Info, stream: &str, version: &Version) -> Result<String> {
		let (_, key) = self.get_name_and_key(stream, version);

		let (_key, s3_url) = get_full_key_and_s3_url(s3_info, &key);

		Ok(s3_url)
	}

	fn get_name_and_key(&self, stream: &str, version: &Version) -> (String, String) {
		// e.g., ...repo/bin_name/target/v0.1.2
		let origin_version_key = format!("{}/{}", self.origin_bin_target_uri(stream), get_version_part(version));

		let gz_name = format!("{}.tar.gz", self.bin_name);
		let gz_key = format!("{}/{}", origin_version_key, gz_name);

		(gz_name, gz_key)
	}
}

// download from local
impl BinRepo {
	async fn download_from_local(
		&self,
		local_repo_origin: &str,
		tmp_dir: &Path,
		stream: &str,
	) -> Result<(String, Version, PathBuf)> {
		// read the version file
		let version = self.get_origin_latest_version(stream).await?;

		let origin_gz = self.get_origin_local_path(local_repo_origin, stream, &version)?;

		let tmp_gz = tmp_dir.join(format!("{}.tar.gz", self.bin_name));
		copy(&origin_gz, &tmp_gz)?;

		let download_path = origin_gz.to_string_lossy().to_string();
		Ok((download_path, version, tmp_gz))
	}

	pub fn get_origin_local_path(&self, local_repo_origin: &str, stream: &str, version: &Version) -> Result<PathBuf> {
		let base_uri = self.origin_bin_target_uri(stream);
		let origin_target_dir = Path::new(local_repo_origin).join(base_uri);

		// e.g., ...repo/bin_name/target/main/v0.1.2/
		let origin_dir = origin_target_dir.join(get_version_part(version));

		// check origin tar file
		let origin_gz = origin_dir.join(format!("{}.tar.gz", self.bin_name));
		if !origin_gz.is_file() {
			return Err(Error::OriginTarGzNotFound(origin_gz.to_string_lossy().to_string()));
		}

		Ok(origin_gz)
	}
}
