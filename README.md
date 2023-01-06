[![Crates.io](https://img.shields.io/crates/v/binst)](https://crates.io/crates/binst)  [![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/jeremychone/rust-binst/blob/master/LICENSE)


> **Under Development** - Working but still early. 

`binst`'s goal is to provide a simple and decentralize mechanism to publish and install \[rust based\] binaries. The command line is repository location agnostic, although we might provide one repository at binst .io at some point. At this point, only support `s3` protocol to publish, and `s3` and `http/https` for install. 


In short: 
- From dev machine, publish with `binst publish -r s3://my_bucket-com/repo` (will publish the `--release` previously built)   
- From any machine, install with `binst install my_cool_lib -r https://my_bucket.com/repo` (assuming you made your bucket a website). 
  - Can also install with same s3 protocol `binst install my_cool_lib -r s3://my_bucket-com/repo`
- One a bin is installed, just do `binst update my_cool_lib` and it will get updated if a new one was deployed in the stream (main by default)
- Stream are based on the semver `pre`, default is `main`.

> OS Support - For now, supports x64 mac and x64 linux gnu. ARM Mac / Linux coming soon. Windows, pull request welcome. 

## Install 

Binary install on the development and install machines

```sh
## On Linux x86 (Intel/AMD)
curl -O https://repo.binst.io/binst/x86_64-unknown-linux-gnu/stable/binst.tar.gz && \
  tar -xvf binst.tar.gz && ./binst self && \
  echo 'source "$HOME/.binst/env"' >> ~/.bashrc && source "$HOME/.binst/env" 


## On Mac Apple Silicon
curl -O https://repo.binst.io/binst/arm64-apple-darwin/stable/binst.tar.gz && \
  tar -xvf binst.tar.gz && ./binst self && \
  echo '\nsource "$HOME/.binst/env"' >> ~/.zshenv && source "$HOME/.binst/env"

## On Mac x86 (Intel/AMD)
curl -O https://repo.binst.io/binst/x86_64-apple-darwin/stable/binst.tar.gz && \
  tar -xvf binst.tar.gz && ./binst self && \
  echo '\nsource "$HOME/.binst/env"' >> ~/.zshenv && source "$HOME/.binst/env"
```

> Note on AWS Linux - When using older instances, you must (and should anyway) update openssl to 1.1 or above with `sudo yum install openssl11` (per default Rust's requirement). 


On the development machine, can use **cargo install** as well

```sh 
cargo install binst
```


## CLI Examples

> Note: You need to have a AWS S3 repository and the credential either as environment variables or as a aws profile

Publishing a binary from dev machine. 

```sh
## Publish to S3 
# Note: publish the --release, with the Cargo.tml version.
binst publish -r s3://my_repo_bucket/repo_root --profile my_aws_profile

# Or with the default AWS credential env variables set
binst publish -r s3://my_repo_bucket/repo_root 
```

Installing the binary published

```sh
# Install the latest stable version published (the one in the info.toml)
binst install cool_cli -r s3://my_repo_bucket/my_repo 

# install from a http/https URL (assuming http domain map  to the s3 bucket above)
binst install cool_cli -r https://my_repo_bucket.com/my_repo

# then, you can run the cool_cli (assuming ~/.binst/bin/ has been added to the PATH)
cool_cli ....

```

> Note: For now a `binst install ...` will reinstall the binary for the latest version. It won't do any semver comparison. 

## Scope and Concepts

- **Not for end users** - `binst` is not for end-users, but for developers and the technical community.
- **Not a package manager** - Dumb is the new smart; use real package manager if dumb is not enough.
- **No Windows support (yet)** - Sorry, I do not have Windows around. Pull request welcome, though.
- **No central repository** - Decentralized first, but eventually will profile one on binst .io for the popular command-line tools. 
- **Simple repo layout** - There can be only one. Also, only .tar.gz format. 
- **Few protocols** - S3 to publish, https/s3 to install. Git planned.


## S3 Credentials - Environment variables

`binst` gets the S3 credentials the following way. 

- if `--profile profile-name` is defined, it will take the standards AWS 
- Otherwise, 
  - first, it will check the `BINST_...` environment variables
    - `BINST_REPO_AWS_KEY_ID` 
    - `BINST_REPO_AWS_KEY_SECRET` 
    - `BINST_REPO_AWS_REGION` 
    - `BINST_REPO_AWS_ENDPOINT` (optional, useful when using minio as object store)
  - Ohterwise, as last fall back, will see the standard `AWS_...` environment variables
    - `AWS_ACCESS_KEY_ID`
    - `AWS_SECRET_ACCESS_KEY`
    - `AWS_DEFAULT_REGION`
    - `AWS_ENDPOINT` (optional, useful when using minio as object store)

## Repo layout

```yaml
- repo_base
  - cool_cli/
    - x86_64-unknown-linux-gnu/      
        - main/ # for main semver, like 0.1.1
            - latest.toml # latest         
            - 0.1.1/
                - cool_cli.toml # package.version = 0.1.1
                - cool_cli.tar.gz
            - 0.1.0/
                - cool_cli.toml # package.version = 0.1.0
                - cool_cli.tar.gz
        - rc/ # when semver has a -..pre.. (take the alpha only).
            - latest.toml
            - 0.1.0-rc.1/
                - cool_cli.toml # package.version = 0.1.0-rc.1
                - cool_cli.tar.gz            

```                

## Local dir layout

```yaml
- ~/.binst/
    - env # sh file to source to set the ~/.binst/bin in the PATH
    - bin/ # symblink dir. Should be in the PATH
        crate_name -> ../packages/crate_name/0.1.3/upacked/crate_name
    - packages/
        - crate_name/
            - 0.1.3/
                - install.toml # (version and repo of the download. Will be user for the future 'binst update' command)
                - crate_name.tar.gz # (downloaded package)
                - unpacked/  # unpacked tar.gz containing the executable crate_name
```

## Targets

- Tested so far:
    - x86_64-apple-darwin (only one tested/supported so far !!!!)
    - x86_64-unknown-linux-gnu
- Plan to test
    - aarch64-apple-darwin
    - aarch64-unknown-linux-gnu
- Not planned (but pull request welcome)
    - x86_64-pc-windows-msvc


https://doc.rust-lang.org/nightly/rustc/platform-support.html

