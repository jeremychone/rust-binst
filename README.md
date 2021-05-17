
**Development starting** Kind of working, still under development. 

`binst`'s goal is to provide a simple way to publish and install Rust-based binaries without the need for cargo-install or OS-dependent package managers' complexity. 

In short, supports publishing the release binary to a custom S3 via env and profile, and installing via publish https/http or S3 via env/profile. 

> Note - There other similar projects. Use what works for you. This one is under heavy development for now.

> OS Support - For now, supports x64 mac and x64 linux gnu. Arm Mac / Linux coming soon. Windows, pull request welcome. 

## Install 

Binary install on the development and install machines

```sh
## On linux 
curl -O https://binst.io/self/binst/x86_64-unknown-linux-gnu/v0.1.0-rc-1/binst.tar.gz && \
  tar -xvf binst.tar.gz && ./binst self && \
  echo 'source "$HOME/.binst/env"' >> ~/.bashrc && source "$HOME/.binst/env" 


## On mac 
curl -O https://binst.io/self/binst/x86_64-apple-darwin/v0.1.0-rc-1/binst.tar.gz && \
  tar -xvf binst.tar.gz && ./binst self && \
  echo '\nsource "$HOME/.binst/env"' >> ~/.zshenv && source "$HOME/.binst/env"
```

> Note on AWS Linux - When using older instances, update openssl to 1.1 with `sudo yum install openssl11`. For now, we do not use `rustls`, and it requires `openssl1.1` (for the better).


On the development machine, can use **cargo install** as well

```sh 
cargo install binst
```



## CLI Examples

> Note: You need to have a AWS S3 repository and the credential either as environment variables or as a aws profile

On the development machine, to publish to a repo.

```sh
## Publish to S3 
# Note: publish the --release, with the Cargo.tml version. This will become the latest stable version
binst publish -r s3://my_repo_bucket/repo_root --profile my_aws_profile
```

On other machines, once binst installed (see below for the binary only version install)

```sh
# Install the latest stable version published (the one in the info.toml)
binst install cool_cli -r s3://mydomain.com/my_repo 

# install from a http/https URL (any dir can be a repo)
binst install cool_cli -r https://

# then, you can run the cool_cli (assuming ~/.binst/bin/ has been added to the PATH)
cool_cli ....

```

> Note: For now a `binst install ...` will reinstall the binary for the latest version. It won't do any semver comparison. 

## Scope and Concepts

- **Not for end users** - `binst` is not for end-users, but for developers and the technical community.
- **Not a package manager** - Dumb is the new smart, use real package manager if dumb is not enough.
- **No Windows support (yet)** - Sorry, do not have one around. Pull request welcome though.
- **No central repository** - Decentralized first.
- **Simple repo layout** - There can be only one.
- **Few protocols** - S3 to publish, https/s3 to install. Git planned


## Repo layout

Assuming the command:

```sh
binst install cool_cli -r s3://my-bucket/my_repo
```

Binst will look at
- Will get the `info.toml` for the `s3://my-bucket/my_repo/cool_cli/[arch-target]/info.toml`
- Will read the `stable.version` (latest stable version) property from the `info.toml`
- Will download the 
  - `s3://my-bucket/my_repo/cool_cli/[arch-target]/v[semver]/cool_cli.tar.gz`
  - In the `$HOMDIR/.binst/packages/cool_cli/v[semver]/cool_cli.tar.gz`
- Unpack the `cool_cli.tar.gz` into `$HOMDIR/.binst/packages/cool_cli/v[semver]/unpacked/`
- Do a symlink from `$HOMDIR/.binst/bin/cool_cli --> $HOMDIR/.binst/packages/cool_cli/v[semver]/unpacked/cool_cli`



## info.toml format

It contains only one version stable, which will be taken into account when doing an install. 
(will allow installing specific version later)

e.g., `s3://my-bucket/my_repo/cool_cli/[arch-target]/info.toml`
```toml
[stable] 
version = "0.3.2"
```

## Local directory store

- ~/.binst/
    - bin/
        crate_name -> ../packages/crate_name/v0.1.3/upacked/crate_name
    - packages/
        - crate_name/
            - v0.1.3/
                install.toml (version and repo of the download. Will be user for the future 'binst update' command)
                crate_name.tar.gz (downloaded package)
                unpacked/crate_name (the executable)


## Targets

- Tested so far:
    - x86_64-apple-darwin (only one tested/supported so far !!!!)
    - x86_64-unknown-linux-gnu
- Plan to test
    - aarch64-apple-darwin
- Not planned (but pull request welcome)
    - aarch64-unknown-linux-gnu
    - x86_64-pc-windows-msvc


https://doc.rust-lang.org/nightly/rustc/platform-support.html

