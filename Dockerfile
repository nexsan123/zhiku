# 智库 Headless Server — Docker build for cloud deployment
# Ubuntu 22.04 has webkit2gtk-4.1 in official repos (required by Tauri v2)

FROM ubuntu:22.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive

# System dependencies for Tauri v2 Linux build
RUN apt-get update && apt-get install -y \
    build-essential pkg-config curl git \
    libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
    librsvg2-dev patchelf libssl-dev libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Node.js 20
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy package files first for better layer caching
COPY package.json package-lock.json* ./
RUN npm install

# Copy Cargo files for dependency caching
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock* src-tauri/
COPY src-tauri/build.rs src-tauri/
# Create dummy src to pre-build deps
RUN mkdir -p src-tauri/src && echo "fn main(){}" > src-tauri/src/main.rs \
    && echo "pub fn run(){}" > src-tauri/src/lib.rs \
    && cd src-tauri && cargo build --release 2>/dev/null || true

# Copy full source
COPY . .

# Build frontend
RUN npm run build

# Build Tauri (release)
RUN cd src-tauri && cargo build --release

# ── Runtime stage ──
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Runtime dependencies only (no -dev packages)
RUN apt-get update && apt-get install -y \
    libwebkit2gtk-4.1-0 libgtk-3-0 libayatana-appindicator3-1 \
    librsvg2-2 libssl3 libsqlite3-0 \
    xvfb ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# App data directory (settings.json + SQLite databases)
RUN mkdir -p /root/.local/share/com.zhiku.app

WORKDIR /app
COPY --from=builder /build/src-tauri/target/release/zhiku /app/zhiku

# Expose QT integration ports
EXPOSE 9601 9600

ENV RUST_LOG=info
ENV DISPLAY=:99

# Start xvfb + zhiku
CMD Xvfb :99 -screen 0 1024x768x24 -nolisten tcp &\
    sleep 1 && \
    /app/zhiku
