
**Development starting** **NO functioning** code yet. 

`binst`'s goal is to provide a simple way to publish and install Rust based binaries without the need of cargo-install and the complexity of OS dependent package managers. 

> Note - There is another interesting project, [binstall](https://crates.io/crates/binstall) which seems to be functioning, but have a different approach. 

## Scope and Concepts

- **Not for end users** - `binst` is not for end-users, but more for developers and the technical community.
- **Not a package manager** - While it does intend to make it trivial to publish and install binaries across operating systems, it is not intended to replace real package managers like homebrew, rpm, dpkg, pacman, .... This is just a simple and dumb way to share your own binaries across oses, no advanced use cases or features.
- **No central repository** - At this point, there is no central plan repository of any kind. Providers and users are responsible for sharing their repository location. No check, no validation, users MUST validate their sources. `binst` just gives a structure and commands to share binaries simply. Anything more complicated should use real package managers.
- **Simple repo structure** - The repository structure is fixed, simple, and does not require anything more than just an https get (not list required) 
- **binst install** - At first will support only **https**, with **git\[hub\]** in a second phase (if asked).
- **binst publish** - At first will support only **AWS S3** with more cloud bucket support if requested. Might plan for a local dir "publish", allowing users to upload it to the location of their choice without worrying about the structure. 


## CLI Examples

First overview

```sh
# setup and install binst under ~/.binst/bin/binst
./binst setup 

# Install the stable version of a deployed binary (See below)
binst install mydomain.com/my_repo/cool_cli

# Install will publish the current binary (--release binary) to the bucket using the .aws/config&credentials profile
# the version and arch-target will be inferred from the current machine/build
binst publish s3://my_repo_bucket --profile my_aws_profile
```

## Remote store structure

Assuming the command:
```sh
binst install mydomain.com/my_repo/cool_cli
```

Binst will look at
- Will get the `info.toml` for the `https://mydomain.com/my_repo/cool_cli/[arch-target]/info.toml`
- Will read the `stable.version` (latest stable version) property from the `info.toml`
- Will download the 
  - `https://mydomain.com/my_repo/cool_cli/[arch-target]/v[semver]/cool_cli.tar.gz`
  - In the `$HOMDIR/.binst/packages/cool_cli/v[semver]/cool_cli.tar.gz`
- Unpack the `cool_cli.tar.gz` into `$HOMDIR/.binst/packages/cool_cli/v[semver]/unpacked/`
- Do a symlink from `$HOMDIR/.binst/bin/cool_cli --> $HOMDIR/.binst/packages/cool_cli/v[semver]/unpacked/cool_cli`

e.g., under the https://mydomain.com/my_repo/cool_cli
```
x86_64-apple-darwin/
    info.toml (latest.version = "0.1.3")
    v0.1.3/
       create_name.tar.gz (the package)
```

## info.toml format

```toml
[stable] # should be the latest stable version
version = "0.3.2"

[xp] # should be the latest experimental/experiment
version = "0.3.4-beta-2"


```

## Local directory store

- ~/.binst/
    config # second phase
    credentials # second phase
    - bin/
        crate_name -> ../packages/crate_name/v0.1.3/upacked/crate_name
    - packages/
        - crate_name/
            - v0.1.3/
                crate_name.tar.gz (downloaded package)
                unpacked/crate_name (the executable)

## Local config

_second phase_

~/.binst-cfg.toml

```toml
[repo.myrepo]
[repo.myrepo.install]
url = "https://my-site-for-repos/my-repos"
key = "some_key"
secret = "some_secret"
profile = "my_aws_profile"
[repo.myrepo.publish]
url = "s3://my_binst_repo_bucket/my_root_path"
key = "some_key"
secret = "some_secret"
profile = "my_aws_profile"

```

## Targets

https://doc.rust-lang.org/nightly/rustc/platform-support.html

- x86_64-apple-darwin
- aarch64-apple-darwin
- x86_64-pc-windows-msvc
- x86_64-unknown-linux-gnu
- aarch64-unknown-linux-gnu

