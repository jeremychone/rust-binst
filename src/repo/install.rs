use crate::{
	paths::binst_package_bin_dir,
	repo::{create_bin_symlink, create_install_toml, make_bin_temp_dir},
	utils::{get_toml_value_as_string, safer_remove_dir},
};
use libflate::gzip::Decoder;
use reqwest;
use std::{
	fs::{copy, create_dir_all, read_to_string, remove_file, File},
	io::{BufReader, Cursor, Read, Write},
	path::{Path, PathBuf},
};
use tar::Archive;
use toml::Value;

use super::{aws_provider::build_new_aws_bucket_client, BinRepo, BinRepoError, Kind, S3Info};

// repo install method(s)
impl BinRepo {
	pub async fn install(&self) -> Result<(), BinRepoError> {
		// create the tempdir
		let tmp_dir = make_bin_temp_dir(&self.bin_name)?;

		//// download the package tar files to the folder
		let (version, tmp_gz) = match &self.kind {
			Kind::Local(local_repo_origin) => self.download_from_local(local_repo_origin, &tmp_dir)?,
			Kind::S3(s3_info) => self.download_from_s3(s3_info, &tmp_dir).await?,
			Kind::Http(base_url) => self.download_from_http(base_url, &tmp_dir).await?,
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
		create_install_toml(&package_dir, &self.repo_raw, &version)?;

		//// create the symlink
		let unpacked_bin = unpacked_dir.join(&self.bin_name);
		let bin_symlink_path = create_bin_symlink(&self.bin_name, &unpacked_bin)?;

		// print info
		println!(
			"Install Complete - package: {} - version: {}
  Downloaded at {}
  Unpacked   at {}
  Symlinked  at {}",
			self.bin_name,
			version,
			gz_path.to_string_lossy(),
			unpacked_dir.to_string_lossy(),
			bin_symlink_path.to_string_lossy()
		);

		safer_remove_dir(&tmp_dir)?;

		Ok(())
	}

	pub async fn get_origin_info_toml_content(&self) -> Result<String, BinRepoError> {
		let base_uri = self.origin_bin_target_uri();

		let content = match &self.kind {
			Kind::Local(local_repo_origin) => {
				let origin_target_dir = Path::new(local_repo_origin).join(base_uri);
				let origin_info_path = origin_target_dir.join("info.toml");
				if !origin_info_path.is_file() {
					return Err(BinRepoError::OriginInfoNotFound(origin_info_path.to_string_lossy().to_string()));
				}
				read_to_string(&origin_info_path)?
			}
			Kind::S3(s3_info) => {
				let S3Info {
					base,
					profile,
					bucket: bucket_name,
				} = s3_info;
				let bucket = build_new_aws_bucket_client(bucket_name, profile).await?;
				let base_key = format!("{}/{}", base, self.origin_bin_target_uri());
				let info_key = &format!("{}/info.toml", base_key);
				let (data, _) = bucket.get_object(info_key).await?;
				let data = std::str::from_utf8(&data).or_else(|_| Err(BinRepoError::InvalidInfoToml(info_key.to_owned())))?;

				data.to_string()
			}
			Kind::Http(base_url) => {
				let target_base_url = format!("{}/{}", base_url, self.origin_bin_target_uri());
				let info_url = &format!("{}/info.toml", target_base_url);
				let resp = reqwest::get(info_url).await?;
				let data = resp.text().await?;
				data
			}
		};

		Ok(content)
	}
}

// download from http
impl BinRepo {
	async fn download_from_http(&self, http_base: &str, tmp_dir: &PathBuf) -> Result<(String, PathBuf), BinRepoError> {
		let http_base = format!("{}/{}", http_base, self.origin_bin_target_uri());

		// download the info.toml
		let info_url = &format!("{}/info.toml", http_base);
		let resp = reqwest::get(info_url).await?;
		let toml = resp.text().await?;
		let toml: Value = toml::from_str(&toml)?;
		let version = get_toml_value_as_string(&toml, &["stable", "version"])?;

		// download the gz
		let gz_name = format!("{}.tar.gz", self.bin_name);
		let gz_url = &format!("{}/v{}/{}", http_base, version, gz_name);
		let resp = reqwest::get(gz_url).await?;
		let gz_tmp_path = tmp_dir.join(gz_name);
		let mut gz_file = File::create(&gz_tmp_path)?;
		let mut content = Cursor::new(resp.bytes().await?);
		std::io::copy(&mut content, &mut gz_file)?;

		Ok((version, gz_tmp_path))
	}
}

// download from s3
impl BinRepo {
	async fn download_from_s3(&self, s3_info: &S3Info, tmp_dir: &PathBuf) -> Result<(String, PathBuf), BinRepoError> {
		let S3Info {
			base,
			profile,
			bucket: bucket_name,
		} = s3_info;

		//// bucket client
		let bucket = build_new_aws_bucket_client(bucket_name, profile).await?;
		let base_key = format!("{}/{}", base, self.origin_bin_target_uri());

		//// download info.toml
		let info_key = &format!("{}/info.toml", base_key);
		let (data, _) = bucket.get_object(info_key).await?;
		let data = std::str::from_utf8(&data).or_else(|_| Err(BinRepoError::InvalidInfoToml(info_key.to_owned())))?;
		let toml: Value = toml::from_str(&data)?;
		let version = get_toml_value_as_string(&toml, &["stable", "version"])?;

		// e.g., ...repo/bin_name/target/v0.1.2
		let origin_version_key = format!("{}/v{}", base_key, version);

		//// download the gz file
		let gz_name = format!("{}.tar.gz", self.bin_name);
		let gz_key = format!("{}/{}", origin_version_key, gz_name);
		let (data, _) = bucket.get_object(gz_key).await?;
		let gz_tmp_path = tmp_dir.join(gz_name);
		let mut gz_file = File::create(&gz_tmp_path)?;
		gz_file.write_all(&data)?;

		//// create the bucket client
		Ok((version, gz_tmp_path))
	}
}

// download from local
impl BinRepo {
	fn download_from_local(&self, local_repo_origin: &str, tmp_dir: &PathBuf) -> Result<(String, PathBuf), BinRepoError> {
		// base orign path the e.g., ..repo/bin_name/target/
		let base_uri = self.origin_bin_target_uri();
		let origin_target_dir = Path::new(local_repo_origin).join(base_uri);

		// get the version file
		let origin_info_path = origin_target_dir.join("info.toml");
		if !origin_info_path.is_file() {
			return Err(BinRepoError::OriginInfoNotFound(origin_info_path.to_string_lossy().to_string()));
		}
		let tmp_info_path = tmp_dir.join("info.toml");
		copy(origin_info_path, &tmp_info_path)?;

		// read the version file
		let toml = read_to_string(&tmp_info_path)?;
		let toml: Value = toml::from_str(&toml)?;
		let version = get_toml_value_as_string(&toml, &["stable", "version"])?;

		// e.g., ...repo/bin_name/target/v0.1.2/
		let origin_dir = origin_target_dir.join(format!("v{}", version));

		// check origin tar file
		let origin_gz = origin_dir.join(format!("{}.tar.gz", self.bin_name));
		if !origin_gz.is_file() {
			return Err(BinRepoError::OriginTarGzNotFound(origin_gz.to_string_lossy().to_string()));
		}

		let tmp_gz = tmp_dir.join(format!("{}.tar.gz", self.bin_name));
		copy(origin_gz, &tmp_gz)?;

		Ok((version.to_string(), tmp_gz))
	}
}
