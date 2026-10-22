# Build image for scripts/desktop-dist.sh: Ubuntu 22.04 (glibc 2.35), the packages the desktop build and the AppImage
# need, and Rust. Built once; the script rebuilds it only if the image is missing or this file changed.
FROM docker.io/library/ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive CARGO_HOME=/cargo RUSTUP_HOME=/cargo/rustup PATH=/cargo/bin:$PATH
RUN apt-get update -qq && apt-get install -y -qq curl ca-certificates file build-essential pkg-config dpkg-dev \
      libdbus-1-dev libgtk-3-dev libayatana-appindicator3-dev libfontconfig1-dev libxkbcommon-dev libxkbcommon-x11-0 \
    && rm -rf /var/lib/apt/lists/*
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal --no-modify-path && chmod -R a+rwX /cargo
