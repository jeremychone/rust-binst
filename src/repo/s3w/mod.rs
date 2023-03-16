//! AWS S3 wrapper

use self::cred::new_aws_client;
use super::S3Info;
use crate::prelude::*;
use crate::repo::Result;
use aws_sdk_s3::types::ByteStream;
use aws_sdk_s3::Client;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, BufReader};
use tokio_stream::StreamExt;

mod cred;

pub struct Bucket {
	client: Client,
}

pub async fn new_repo_bucket(profile: Option<String>) -> Result<Bucket> {
	let client = new_aws_client(profile).await?;
	Ok(Bucket { client })
}

impl Bucket {
	/// Get the key object body as string
	pub async fn download_to_string(&self, s3_info: &S3Info, key: &str) -> Result<String> {
		let (key, _) = get_full_key_and_s3_url(s3_info, key);
		let req = self.client.get_object().bucket(s3_info.bucket.clone()).key(key);
		let res = req.send().await?;
		let stream = res.body;

		// --- Read the buffer all at once (assume small content)
		let mut buf_reader = BufReader::new(stream.into_async_read());
		let mut content = String::new();
		buf_reader.read_to_string(&mut content).await?;

		Ok(content)
	}

	/// Download a key relative to the bucket/root, to a file_path
	/// Returns the resolved S3 URL
	pub async fn download_to_file(&self, s3_info: &S3Info, key: &str, file_path: &Path) -> Result<String> {
		let (key, s3_url) = get_full_key_and_s3_url(s3_info, key);

		let req = self.client.get_object().bucket(s3_info.bucket.clone()).key(&key);
		let res = req.send().await?;
		let mut data: ByteStream = res.body;

		// Streaming
		let file = File::create(file_path)?;
		let mut buf_writer = BufWriter::new(file);
		while let Some(bytes) = data.try_next().await? {
			buf_writer.write_all(&bytes)?;
		}
		buf_writer.flush()?;

		Ok(s3_url)
	}

	pub async fn upload_text(
		&self,
		s3_info: &S3Info,
		key: &str,
		content: String,
		content_type: Option<&str>,
	) -> Result<String> {
		let (key, s3_url) = get_full_key_and_s3_url(s3_info, key);
		let content_type = content_type.unwrap_or("text/plain");
		let body = ByteStream::from(content.into_bytes());

		// BUILD - aws s3 put request
		let builder = self
			.client
			.put_object()
			.key(&key)
			.bucket(&s3_info.bucket)
			.body(body)
			.content_type(content_type);

		// EXECUTE - aws request
		builder.send().await?;

		Ok(s3_url)
	}

	pub async fn upload_file(&self, s3_info: &S3Info, key: &str, file_path: &Path) -> Result<String> {
		let (key, s3_url) = get_full_key_and_s3_url(s3_info, key);
		let mime_type = mime_guess::from_path(file_path).first_or_octet_stream().to_string();
		let file_path = PathBuf::from(file_path);
		let body = ByteStream::from_path(&file_path).await?;
		// BUILD - aws s3 put request
		let builder = self
			.client
			.put_object()
			.key(&key)
			.bucket(&s3_info.bucket)
			.body(body)
			.content_type(mime_type);

		// EXECUTE - aws request
		builder.send().await?;

		Ok(s3_url)
	}
}

pub fn get_full_key_and_s3_url(s3_info: &S3Info, key: &str) -> (String, String) {
	let full_key = if s3_info.base.is_empty() {
		key.to_string()
	} else {
		f!("{}/{key}", s3_info.base)
	};
	let s3_url = f!("s3://{}/{key}", s3_info.bucket);
	(full_key, s3_url)
}
