# Multi-stage Dockerfile for building Frappe (Tauri + React + Rust)
# Targets Linux; use native runners for macOS/Windows builds

# Stage 1: Frontend build
FROM node:22-alpine AS frontend-builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Stage 2: Rust build
FROM rust:latest as rust-builder
WORKDIR /app

# Install system dependencies required for Tauri on Linux
RUN apt-get update && apt-get install -y \
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libdbus-1-dev \
    pkg-config \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js (needed for Tauri CLI)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Copy source
COPY . .

# Copy built frontend from previous stage
COPY --from=frontend-builder /app/dist ./dist

# Install npm dependencies
RUN npm ci

# Build Tauri app
RUN npx @tauri-apps/cli build --target x86_64-unknown-linux-gnu

# Stage 3: Runtime (output binaries)
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libwebkit2gtk-4.1-1 \
    libgtk-3-0 \
    libayatana-appindicator3-1 \
    librsvg2-2 \
    libdbus-1-3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy built binary and bundled assets
COPY --from=rust-builder /app/src-tauri/target/x86_64-unknown-linux-gnu/release/frappe /usr/local/bin/frappe

# Metadata
LABEL org.opencontainers.image.title="Frappe"
LABEL org.opencontainers.image.description="Print production management & PDF tooling"
LABEL org.opencontainers.image.url="https://github.com/mbx30/frappe"

ENTRYPOINT ["frappe"]
