# 🤖 OpenClaw Edge AI Platform

<div align="center">

[![Release](https://img.shields.io/github/v/release/markfietje/jetson-openclaw-setup?style=for-the-badge&logo=github)](https://github.com/markfietje/jetson-openclaw-setup/releases)
[![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue?style=for-the-badge)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-ARM64-9cf?style=for-the-badge&logo=linux)](https://github.com/markfietje/jetson-openclaw-setup/releases)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)

**Production-ready AI assistant infrastructure for Ubuntu/Debian Linux**

[Features](#-features) • [Supported Platforms](#-supported-platforms) • [Quick Start](#-quick-start) • [Installation](#-installation) • [Contributing](#-contributing)

</div>

---

## 📋 Overview

OpenClaw AI Assistant Infrastructure provides a solid, well-built monorepo for deploying AI assistant infrastructure on Linux. It includes a knowledge graph engine, Signal messaging bridge, and seamless OpenClaw integration.

**What you get:**
- 🧠 **Brain Server** - Knowledge graph + semantic search engine
- 📡 **Signal Gateway** - Signal ↔ OpenClaw bridge with auto-retry
- 📦 **Debian Packages** - Easy installation via .deb packages
- 🔒 **Security Hardened** - Systemd isolation, CORS protection, input validation
- 🚀 **ARM64 Optimized** - Built for edge AI devices (Jetson Nano, Raspberry Pi)

---

## 🎯 Supported Platforms

### ARM64 (Recommended)
Pre-built packages for edge AI devices:

| Platform | Status |
|----------|--------|
| **NVIDIA Jetson Nano** | ✅ Tested |
| **NVIDIA Jetson Xavier/Orin** | ✅ Compatible |
| **Raspberry Pi 4/5** | ✅ Compatible |
| **Other ARM64 Linux** | ✅ Should work |

### AMD64 (x86_64) - Build from Source
Pre-built packages not provided. Build from source for other architectures:

```bash
git clone https://github.com/markfietje/jetson-openclaw-setup.git
cd jetson-openclaw-setup/services/brain-server
cargo build --release
```

---

## ✨ Features

### 🧠 Brain Server

**Knowledge Graph & Semantic Search Engine**

- 🔍 Semantic search with model2vec-rs (minishlab/potion-retrieval-32M)
- 🕸️ Knowledge graph with entity/relationship extraction
- 🎯 Graph traversal with configurable depth (max 3)
- 🛡️ Prompt injection detection for security
- 🔌 RESTful API: health, stats, search, ingest, graph/*
- ⚡ <1ms query speed
- 💾 SQLite database with connection pooling

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

- **Hardware:** ARM64 Linux system (Jetson Nano, Raspberry Pi, etc.)
- **Software:** Ubuntu/Debian, systemd

### Installation

**Download and install:**

```bash
# Download latest ARM64 packages
wget https://github.com/markfietje/jetson-openclaw-setup/releases/latest/download/brain-server_0.8.1_arm64.deb
wget https://github.com/markfietje/jetson-openclaw-setup/releases/latest/download/signal-gateway_0.1.1_arm64.deb

# Install packages
sudo dpkg -i brain-server_0.8.1_arm64.deb
sudo dpkg -i signal-gateway_0.1.1_arm64.deb

# Services auto-start and enable
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

Good security practices built-in:

- ✅ **Loopback-only binding** - Services only accessible locally
- ✅ **Systemd hardening** - NoNewPrivileges, ProtectSystem, ProtectHome
- ✅ **CORS protection** - Environment-based origin validation
- ✅ **Input validation** - SQL injection prevention, message validation
- ✅ **Rate limiting** - DoS protection
- ✅ **Dedicated users** - Services run as isolated system users

---

## 📊 Performance

### Brain Server
- **Memory:** ~150 MB
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

# Build for ARM64 (requires cross-compilation tools)
./scripts/build-deb-packages.sh

# Or build individual services
cd services/brain-server
cargo build --release --target aarch64-unknown-linux-gnu  # ARM64
cargo build --release --target x86_64-unknown-linux-gnu   # AMD64
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

## 🤝 Contributing

Contributions welcome! Here's how:

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

Licensed under [Apache License, Version 2.0](LICENSE-APACHE)

---

## 👤 Author

**Mark Fietje**
- GitHub: [@markfietje](https://github.com/markfietje)
- X: [@mark_fietje](https://x.com/mark_fietje)

---

## 🙏 Acknowledgments

- [OpenClaw](https://github.com/openclaw) - AI assistant framework
- [model2vec-rs](https://github.com/leeeeeeeem/model2vec-rs) - Embedding engine
- [presage](https://github.com/whisperfish/presage) - Signal library
- Raspberry Pi Foundation
- NVIDIA Jetson Community

---

<div align="center">

**Built with ❤️ and ☕**

**Works on Jetson Nano, Raspberry Pi, and any Linux system**

**[⬆ Back to Top](#-openclaw-ai-assistant-infrastructure)**

</div>