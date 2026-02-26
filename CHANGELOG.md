# Changelog

All notable changes to Jetson OpenClaw Setup will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

---

## [0.8.4] - 2026-02-26

### Infrastructure
- Simplified to ARM64-only builds (Jetson Nano focus)
- All containers now use 4GB memory
- Removed AMD64 builds - users can build from source
- macOS runner is stateless (no local dependencies)

### CI/CD
- Skipped pre-release tests in release workflow (already run in CI)
- Uses containers for: binary building, Debian packaging, checksums

---

## [0.8.3] - 2026-02-26

### Infrastructure
- Added Enterprise-Grade workflow names
- Removed "(Enterprise-Grade)" suffix for cleaner naming

---

## [0.8.2] - 2025-02-25

### Infrastructure
#### CI/CD Improvements
- **Switched to Apple Containers** for building Linux binaries
- All workflows now use `apple-container` runner (self-hosted on MacBook Pro M1 Pro)
- ARM64 builds: Native Linux ARM64 containers on Apple Silicon
- AMD64 builds: Linux AMD64 containers with Rosetta emulation
- Eliminated cross-compilation issues with C dependencies

#### Build System
- Building inside official Rust containers (`rust:latest`)
- Full Linux environment for both ARM64 and AMD64 targets
- Proper support for C dependencies (openssl, protobuf, etc.)
- Binary verification and checksums for all builds

### Brain Server
- Optimized builds for ARM64 (Jetson Nano) and AMD64
- Debian packages included for easy installation
- All tests passing

### Signal Gateway
- **Fixed code formatting issues** in signal-gateway/src/api/mod.rs
- Applied cargo fmt for consistent code style
- Zero clippy warnings
- Optimized builds for ARM64 (Jetson Nano) and AMD64
- Debian packages included for easy installation
- All tests passing

### Artifacts
- **brain-server_arm64.deb** - Debian package for Jetson Nano
- **brain-server_amd64.deb** - Debian package for standard Linux
- **signal-gateway_arm64.deb** - Debian package for Jetson Nano
- **signal-gateway_amd64.deb** - Debian package for standard Linux
- Binary tarballs with SHA256 checksums

---

## [0.8.1] - 2026-02-24
