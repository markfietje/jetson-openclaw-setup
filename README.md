# 🤖 Jetson OpenClaw Setup

<div align="center">

[![Release](https://img.shields.io/github/v/release/markfietje/jetson-openclaw-setup?style=for-the-badge&logo=github)](https://github.com/markfietje/jetson-openclaw-setup/releases)
[![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue?style=for-the-badge)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Jetson%20Nano-green?style=for-the-badge&logo=nvidia)](https://developer.nvidia.com/embedded/jetson-nano)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)

**Enterprise-grade AI assistant infrastructure for NVIDIA Jetson Nano**

[Features](#-features) • [Quick Start](#-quick-start) • [Installation](#-installation) • [Documentation](#-documentation) • [Contributing](#-contributing)

</div>

---

## 📋 Overview

Jetson OpenClaw Setup provides a production-ready monorepo for deploying AI assistant infrastructure on NVIDIA Jetson Nano. It includes a knowledge graph engine, Signal messaging bridge, and seamless OpenClaw integration.

**Key Highlights:**
- 🧠 **Brain Server** - Knowledge graph + semantic search engine
- 📡 **Signal Gateway** - Signal ↔ OpenClaw bridge with auto-retry
- 📦 **Debian Packages** - Professional package management
- 🔒 **Security Hardened** - Systemd isolation, CORS protection, input validation
- ⚡ **Optimized** - ARM64 Cortex-A57 tuned binaries

---

## ✨ Features

### 🧠 Brain Server

**Knowledge Graph & Semantic Search Engine**

- 📚 1,293 knowledge entries with 384-dimensional embeddings
- 🕸️ 461 entities + 779 relationships in knowledge graph
- 🔍 Semantic search with model2vec-rs (minishlab/potion-retrieval-32M)
- 🎯 Graph traversal with configurable depth (max 3)
- 🛡️ Prompt injection detection for security
- 🔌 RESTful API: health, stats, search, ingest, graph/*
- ⚡ <1ms query speed, 25% memory usage

### 📡 Signal Gateway

**Signal Messaging Bridge**

- 🔄 Automatic receiver startup with 5-retry exponential backoff
- 📞 Phone number → UUID resolution with caching
- 🌐 HTTP + JSON-RPC API for sending messages
- 📡 SSE message stream for real-time receiving
- ⚙️ Rate limiting and input validation
- 🚀 ~10MB memory, 2s startup, 83ms shutdown

---

## 🚀 Quick Start

### Prerequisites

- **Hardware:** NVIDIA Jetson Nano (ARM64)
- **Software:** Debian/Ubuntu, systemd
- **Network:** SSH access configured

### Installation

**Option 1: Debian Package (Recommended)**

```bash
# Download latest release
wget https://github.com/markfietje/jetson-openclaw-setup/releases/latest/download/brain-server_0.8.1_arm64.deb
wget https://github.com/markfietje/jetson-openclaw-setup/releases/latest/download/signal-gateway_0.1.1_arm64.deb

# Install packages
sudo dpkg -i brain-server_0.8.1_arm64.deb
sudo dpkg -i signal-gateway_0.1.1_arm64.deb

# Services auto-start and enable
```

**Option 2: Binary Installation**

```bash
# Download and extract
wget https://github.com/markfietje/jetson-openclaw-setup/releases/latest/download/brain-server-arm64.tar.gz
tar xzf brain-server-arm64.tar.gz
sudo mv brain-server /usr/local/bin/

# Install systemd service (manual setup required)
sudo systemctl enable brain-server
sudo systemctl start brain-server
```

### Verify Installation

```bash
# Check service status
sudo systemctl status brain-server signal-gateway

# Test health endpoints
curl http://localhost:8765/health     # Brain Server
curl http://localhost:8080/v1/health  # Signal Gateway
```

---

## 📦 Services

| Service | Version | Port | Purpose |
|---------|---------|------|---------|
| 🧠 Brain Server | v0.8.1 | 8765 | Knowledge graph & semantic search |
| 📡 Signal Gateway | v0.1.1 | 8080 | Signal messaging bridge |

---

## 🔧 Configuration

### Brain Server

**Config:** `/etc/brain-server/config.toml`

```toml
[server]
host = "127.0.0.1"
port = 8765

[database]
path = "/var/lib/brain-server/db/brain.db"

[embedding]
model = "minishlab/potion-retrieval-32M"
```

### Signal Gateway

**Config:** `/etc/signal-gateway/config.toml`

```toml
[server]
host = "127.0.0.1"
port = 8080

[signal]
data_dir = "/var/lib/signal-gateway/signal-data"
# phone_number = "+1234567890"  # Required for first-time setup

[brain_server]
url = "http://127.0.0.1:8765"
```

---

## 🛡️ Security

- ✅ **Loopback-only binding** - No internet exposure
- ✅ **Systemd hardening** - NoNewPrivileges, ProtectSystem, ProtectHome
- ✅ **CORS protection** - Environment-based origin validation
- ✅ **Input validation** - SQL injection prevention, message validation
- ✅ **Rate limiting** - DoS protection
- ✅ **Dedicated users** - Service isolation with system users
- ✅ **Security audit** - A+ rating

---

## 📊 Performance

### Brain Server
- **Memory:** 1,043 MB (25% of 4 GB)
- **Query Speed:** <1ms per search
- **Database:** ~10 MB (compressed, indexed)
- **Startup:** ~3 seconds

### Signal Gateway
- **Memory:** ~10 MB
- **Startup:** ~2 seconds
- **Shutdown:** 83 ms
- **Retry Logic:** 5 attempts with exponential backoff

---

## 🔨 Development

### Build from Source

```bash
# Clone repository
git clone https://github.com/markfietje/jetson-openclaw-setup.git
cd jetson-openclaw-setup

# Build ARM64 binaries (requires cross-compilation tools)
./scripts/build-deb-packages.sh

# Or build individual services
cd services/brain-server
cargo build --release --target aarch64-unknown-linux-gnu
```

### Run Tests

```bash
# Brain Server
cd services/brain-server
cargo test --release
cargo clippy -- -D warnings

# Signal Gateway
cd services/signal-gateway
cargo test --release
cargo clippy -- -D warnings
```

---

## 📚 Documentation

- 📖 [**CHANGELOG.md**](CHANGELOG.md) - Version history and release notes
- 📦 [**packages/README.md**](packages/README.md) - Debian package guide
- 🚀 [**docs/RELEASE-WORKFLOW.md**](docs/RELEASE-WORKFLOW.md) - Release process
- 🔧 [**API Documentation**](docs/API.md) - API endpoints and usage

---

## 🚢 Deployment

### Automated Deployment

Push a tag to trigger the release workflow:

```bash
git tag v0.9.0
git push origin v0.9.0
```

The workflow will:
1. ✅ Run tests and validation
2. 📝 Generate comprehensive changelog
3. 🏗️ Build ARM64 binaries
4. 📦 Create Debian packages
5. 🚀 Deploy to GitHub Releases
6. 🎯 Deploy to Jetson Nano (automatic)

### Manual Deployment

```bash
# Copy package to Jetson
scp brain-server_0.8.1_arm64.deb jetson@jetson:/tmp/

# Install on Jetson
ssh jetson@jetson "sudo dpkg -i /tmp/brain-server_0.8.1_arm64.deb"
```

---

## 🤝 Contributing

We welcome contributions! Please see our contributing guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'feat: add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

**Code Standards:**
- ✅ Zero clippy warnings (`cargo clippy -- -D warnings`)
- ✅ Formatted code (`cargo fmt -- --check`)
- ✅ All tests passing (`cargo test`)
- ✅ Update documentation

---

## 📄 License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

## 📈 Status

| Component | Status | Version |
|-----------|--------|---------|
| Brain Server | ✅ Production Ready | v0.8.1 |
| Signal Gateway | ✅ Production Ready | v0.1.1 |
| CI/CD Pipeline | ✅ Active | - |
| Security Audit | ✅ A+ Rating | - |

---

## 👤 Author

**Mark Fietje**
- GitHub: [@markfietje](https://github.com/markfietje)

---

## 🙏 Acknowledgments

- [OpenClaw](https://github.com/openclaw) - AI assistant framework
- [model2vec-rs](https://github.com/leeeeeeeem/model2vec-rs) - Embedding engine
- [presage](https://github.com/whisperfish/presage) - Signal library
- NVIDIA Jetson Community

---

<div align="center">

**Built with ❤️ and ☕ on Jetson Nano**

**[⬆ Back to Top](#-jetson-openclaw-setup)**

</div>