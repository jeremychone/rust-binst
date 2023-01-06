use crate::exec::CARGO_TOML;
use crate::repo::s3w::new_repo_bucket;
use crate::repo::{extract_stream, get_version_part, RepoInfo, S3Info};
use crate::utils::{clean_path, exec_cmd_args, get_toml_value_as_string, safer_remove_dir};
use libflate::gzip::Encoder;
use semver::Version;
use std::fs::{copy, create_dir, create_dir_all, read_to_string, write, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use tar::Builder;
use toml::Value;

use super::{get_release_bin, make_bin_temp_dir, BinRepo, Error};

#[derive(Debug)]
struct UploadRec {
	latest_toml: PathBuf,
	gz: PathBuf,
	package_toml: PathBuf,
	version: Version,
	stream: String,
	at_path: Option<String>,
}

// repo main publish method
impl BinRepo {
	pub async fn publish(&self, at_path: Option<String>) -> Result<(), Error> {
		let bin_name = &self.bin_name;

		// create the temp dir
		let tmp_dir = make_bin_temp_dir(bin_name)?;

		// read the Cargo.toml version
		let toml = read_to_string(CARGO_TOML)?;
		let toml: Value = toml::from_str(&toml)?;
		let version = get_toml_value_as_string(&toml, &["package", "version"])?;
		let version = Version::parse(&version).unwrap();

		let stream = extract_stream(&version);

		println!(
			"Publishing package: {}  |  version: {}  |  to: {}",
			bin_name,
			version,
			self.publish_repo.url()
		);

		let mut build_args = vec!["build", "--release"];
		if let Some(target) = &self.target {
			build_args.push("--target");
			build_args.push(target);
		}
		exec_cmd_args("cargo", &build_args)?;
		// build the package

		// get the release bin path
		let bin_file = get_release_bin(bin_name, &self.target)?;

		// get the file to pack in the tmp_dir/to_pack folder
		// TODO: support multiple files
		let to_pack_dir = tmp_dir.join("to_pack");
		create_dir(&to_pack_dir)?;
		let to_pack_file = to_pack_dir.join(bin_name);
		copy(&bin_file, &to_pack_file)?;

		// create the latest file
		let latest_toml_path = tmp_dir.join("latest.toml");
		write(&latest_toml_path, create_latest_toml_content(&version))?;

		// create the package file
		let package_toml_path = tmp_dir.join("package.toml");
		let package_content = create_package_toml_content(bin_name, &stream, &at_path, &version);
		write(&package_toml_path, package_content)?;

		println!("   packing: {}", to_pack_file.to_string_lossy());
		// create tar
		let tar_name = format!("{}.tar", bin_name);
		let tar_path = tmp_dir.join(&tar_name);
		let tar_file = File::create(&tar_path).unwrap();
		let mut tar_file = Builder::new(tar_file);
		tar_file.append_file(&self.bin_name, &mut File::open(bin_file)?)?;

		// create gz
		let gz_name = format!("{}.gz", &tar_name);
		let tar_file = File::open(&tar_path).unwrap();
		let mut reader = BufReader::new(tar_file);
		let mut encoder = Encoder::new(Vec::new()).unwrap();
		std::io::copy(&mut reader, &mut encoder).unwrap();
		let encoded_data = encoder.finish().into_result().unwrap();
		let gz_path = tmp_dir.join(gz_name);
		let mut gz_file = File::create(gz_path.as_path()).unwrap();
		gz_file.write_all(&encoded_data)?;
		println!("    packed: {}", gz_path.to_string_lossy());

		// start the upload
		let rec = UploadRec {
			latest_toml: latest_toml_path,
			gz: gz_path,
			version,
			stream: stream.to_string(),
			package_toml: package_toml_path,
			at_path,
		};

		match &self.publish_repo {
			RepoInfo::Local(local_repo) => self.upload_to_local(local_repo, rec)?,
			RepoInfo::S3(s3_info) => self.upload_to_s3(s3_info, rec).await?,
			RepoInfo::Http(_) => return Err(Error::HttpProtocolNotSupportedForPublish),
		};

		// TODO - needds to make sure clean dir even if error above. Wrap in function.
		safer_remove_dir(&tmp_dir)?;

		Ok(())
	}
}

// upload to local
impl BinRepo {
	fn upload_to_local(&self, origin_repo: &str, upload_rec: UploadRec) -> Result<(), Error> {
		let UploadRec {
			version,
			latest_toml,
			gz: gz_file_path,
			stream,
			package_toml,
			at_path,
		} = upload_rec;

		let is_at_path = at_path.is_some();
		let path_or_stream = at_path.unwrap_or(stream);
		let origin_target_dir = Path::new(origin_repo).join(self.origin_bin_target_uri(&path_or_stream));

		if !origin_target_dir.is_file() {
			create_dir_all(&origin_target_dir)?;
		}

		//// copy the latest.toml
		if !is_at_path {
			let origin_info_path = origin_target_dir.join("latest.toml");
			copy(latest_toml, &origin_info_path)?;
			println!("    copied: {}", origin_info_path.to_string_lossy());
		}

		//// build the package dir for version or at_path
		let package_dir = if is_at_path {
			origin_target_dir
		} else {
			origin_target_dir.join(get_version_part(&version))
		};
		if !package_dir.is_file() {
			create_dir_all(&package_dir)?;
		}

		//// copy the gz file
		let origin_gz_path = package_dir.join(format!("{}.tar.gz", self.bin_name));
		copy(gz_file_path, &origin_gz_path)?;
		println!("    copied: {}", origin_gz_path.to_string_lossy());

		//// copy the package toml
		let origin_package_path = package_dir.join(format!("{}.toml", self.bin_name));
		copy(package_toml, &origin_package_path)?;
		println!("    copied: {}", origin_package_path.to_string_lossy());

		Ok(())
	}
}

// upload to s3
impl BinRepo {
	async fn upload_to_s3(&self, s3_info: &S3Info, upload_rec: UploadRec) -> Result<(), Error> {
		let bin_name = &self.bin_name;

		let UploadRec {
			version,
			latest_toml,
			gz: gz_file_path,
			stream,
			package_toml: package_toml_path,
			at_path,
		} = upload_rec;

		let is_at_path = at_path.is_some();
		let path_or_stream = at_path.unwrap_or(stream);
		let origin_target_key = self.origin_bin_target_uri(&path_or_stream);

		//// Create the bucket client
		// let bucket = build_new_aws_bucket_client(bucket_name, profile).await?;

		let bucket = new_repo_bucket(s3_info.profile.clone()).await?;

		//// Upload latest.toml
		if !is_at_path {
			let latest_key = clean_path(format!("{}/latest.toml", origin_target_key));
			let content = read_to_string(latest_toml)?;

			let s3_url = bucket.upload_text(s3_info, &latest_key, content, None).await?;
			println!("  uploaded: {s3_url}");
		}

		//// build the package key
		let package_key = if is_at_path {
			origin_target_key
		} else {
			format!("{}/{}", origin_target_key, get_version_part(&version))
		};

		//// Upload the package gz
		let gz_key = clean_path(format!("{}/{}.tar.gz", package_key, bin_name));
		// TODO: need to stream content
		let url = bucket.upload_file(s3_info, &gz_key, &gz_file_path).await?;
		println!("  uploaded: {url}");

		//// Upload the package toml
		let package_key = clean_path(format!("{}/{}.toml", package_key, bin_name));
		let content = read_to_string(package_toml_path)?;
		let url = bucket.upload_text(s3_info, &package_key, content, None).await?;
		println!("  uploaded: {url}");

		Ok(())
	}
}

fn create_package_toml_content(bin_name: &str, stream: &str, path: &Option<String>, version: &Version) -> String {
	let mut content = format!(
		r#"[package]
name = "{}"		
stream = "{}"
version = "{}"
"#,
		bin_name, stream, version
	);

	if let Some(path) = path {
		content.push_str(&format!("path = \"{}\"\n", path));
	}

	content
}

fn create_latest_toml_content(version: &Version) -> String {
	format!("[latest]\nversion = \"{}\"", version)
}
