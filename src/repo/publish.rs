use std::{
	fs::{copy, create_dir, create_dir_all, read_to_string, write, File},
	io::{BufReader, Read, Write},
	path::{Path, PathBuf},
};

use libflate::gzip::Encoder;
use tar::Builder;
use toml::Value;

use crate::{
	exec::CARGO_TOML,
	paths::os_target,
	repo::{aws_provider::build_new_aws_bucket_client, Kind, S3Info},
	utils::{get_toml_value_as_string, safer_remove_dir},
};

use super::{get_release_bin, make_bin_temp_dir, BinRepo, BinRepoError};

struct UploadRec {
	info: PathBuf,
	gz: PathBuf,
	version: String,
}

// repo main publish method
impl BinRepo {
	pub async fn publish(&self) -> Result<(), BinRepoError> {
		// create the temp dir
		let tmp_dir = make_bin_temp_dir(&self.bin_name)?;

		// read the Cargo.toml version
		let toml = read_to_string(CARGO_TOML)?;
		let toml: Value = toml::from_str(&toml)?;
		let version = get_toml_value_as_string(&toml, &["package", "version"])?;

		println!("Publishing package: {}  |  version: {}  |  to: {}", &self.bin_name, version, &self.repo_raw);

		// get the release bin path
		let bin_file = get_release_bin(&self.bin_name)?;

		// get the file to pack in the tmp_dir/to_pack folder
		// TODO: support multiple files
		let to_pack_dir = tmp_dir.join("to_pack");
		create_dir(&to_pack_dir)?;
		let to_pack_file = to_pack_dir.join(&self.bin_name);
		copy(&bin_file, &to_pack_file)?;

		// create the info file
		let info_file = tmp_dir.join("info.toml");
		write(&info_file, get_info_toml(&version))?;

		println!("   packing: {}", to_pack_file.to_string_lossy());
		// create tar
		let tar_name = format!("{}.tar", &self.bin_name);
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
			info: info_file,
			gz: gz_path,
			version: version.to_string(),
		};

		match &self.kind {
			Kind::Local(local_repo) => self.upload_to_local(local_repo, rec)?,
			Kind::S3(s3_info) => self.upload_to_s3(s3_info, rec).await?,
			Kind::Http(_) => return Err(BinRepoError::HttpProtocolNotSupportedForPublish),
		};

		safer_remove_dir(&tmp_dir)?;

		Ok(())
	}
}

// upload to local
impl BinRepo {
	fn upload_to_local(&self, origin_repo: &str, upload_rec: UploadRec) -> Result<(), BinRepoError> {
		let target = os_target();
		let origin_target_dir = Path::new(origin_repo).join(&self.bin_name).join(target);

		let UploadRec {
			version,
			info: info_file_path,
			gz: gz_file_path,
		} = upload_rec;

		if !origin_target_dir.is_file() {
			create_dir_all(&origin_target_dir)?;
		}

		// copy the info.toml
		let origin_info_path = origin_target_dir.join("info.toml");
		copy(info_file_path, &origin_info_path)?;
		println!("    copied: {}", origin_info_path.to_string_lossy());
		// commit the gz file
		let version_dir = origin_target_dir.join(format!("v{}", version));
		if !version_dir.is_file() {
			create_dir_all(&version_dir)?;
		}
		let origin_gz_path = version_dir.join(format!("{}.tar.gz", self.bin_name));
		copy(gz_file_path, &origin_gz_path)?;
		println!("    copied: {}", origin_gz_path.to_string_lossy());

		Ok(())
	}
}

// upload to s3
impl BinRepo {
	async fn upload_to_s3(&self, s3_info: &S3Info, upload_rec: UploadRec) -> Result<(), BinRepoError> {
		let bin_name = &self.bin_name;

		let UploadRec {
			version,
			info: info_file_path,
			gz: gz_file_path,
		} = upload_rec;

		let S3Info {
			base,
			profile,
			bucket: bucket_name,
		} = s3_info;

		//// Create the bucket client
		let bucket = build_new_aws_bucket_client(bucket_name, profile).await?;

		let base_key = format!("{}/{}", base, self.origin_bin_target_uri());

		//// Upload info.toml
		let info_key = format!("{}/info.toml", base_key);
		let mut info_file = File::open(&info_file_path)?;
		let mut buffer = String::new();
		info_file.read_to_string(&mut buffer)?;
		bucket.put_object_with_content_type(&info_key, buffer.as_bytes(), "text/plain").await?;
		println!("  uploaded: s3:://{}/{}", bucket_name, info_key);

		//// Upload the package js
		let gz_name = format!("{}.tar.gz", bin_name);
		let gz_key = format!("{}/v{}/{}", base_key, version, gz_name);
		// TODO: need to stream content
		let mut gz_file = File::open(&gz_file_path)?;
		let mut buffer = Vec::new();
		gz_file.read_to_end(&mut buffer)?;
		bucket.put_object(&gz_key, &buffer).await?;

		println!("  uploaded: s3:://{}/{}", bucket_name, gz_key);

		Ok(())
	}
}

fn get_info_toml(version: &str) -> String {
	format!(
		r#"
[stable]
version = "{}"
"#,
		version
	)
}
