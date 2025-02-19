FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

# Include lld linker to improve build times either by using environment variable
# RUSTFLAGS="-C link-arg=-fuse-ld=lld" or with Cargo's configuration file (i.e see .cargo/config.toml).
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
   && apt-get -y install clang lld \
   && apt-get autoremove -y && apt-get clean -y

# there was a issue with permission when running cargo run as vscode, probably couldnt access files located under /usr/local/cargo
# e.g.:
# error: failed to open `/usr/local/cargo/registry/cache/index.crates.io-6f17d22bba15001f/http-body-1.0.1.crate`
# 
# Caused by:
#   Permission denied (os error 13)
USER vscode
RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres
