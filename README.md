# Jetson OpenClaw Setup 🤖

Monorepo for Mark's AI assistant infrastructure, including brain-server, signal-gateway, and OpenClaw configurations.

## 📦 Services

### 🧠 Brain Server
- **Language:** Rust
- **Purpose:** Knowledge graph + semantic search engine
- **Features:** 
  - 1,293+ knowledge entries
  - Entity extraction and relationship detection
  - 384-dimensional embeddings (model2vec-rs)
  - Knowledge graph with 461 entities + 779 relationships

### 📡 Signal Gateway
- **Language:** Rust
- **Purpose:** Bridge between Signal messaging and OpenClaw
- **Features:**
  - Automatic receiver startup with 5-retry system
  - Exponential backoff (5s → 80s)
  - Clean shutdown (83ms)
  - HTTP + JSON-RPC API
  - Phone number → UUID resolution

### ⚙️ OpenClaw Config
- **Purpose:** AI assistant configuration
- **Channels:** Signal (enabled), WhatsApp (configurable)
- **Model:** zai/glm-4.7

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ (for brain-server and signal-gateway)
- Linux ARM64 (Jetson Nano) or compatible
- OpenClaw installed

### Installation

1. **Clone repository:**
   ```bash
   git clone https://github.com/[USERNAME]/jetson-openclaw-setup.git
   cd jetson-openclaw-setup
   ```

2. **Install Brain Server:**
   ```bash
   cd services/brain-server
   cargo build --release
   sudo cp target/release/brain-server /usr/local/bin/
   ```

3. **Install Signal Gateway:**
   ```bash
   cd services/signal-gateway
   cargo build --release
   sudo cp target/release/signal-gateway /usr/local/bin/
   ```

4. **Install wrapper script:**
   ```bash
   sudo cp scripts/signal-gateway-wrapper.sh /usr/local/bin/
   sudo chmod +x /usr/local/bin/signal-gateway-wrapper.sh
   ```

5. **Install systemd service:**
   ```bash
   sudo cp services/openclaw-config/signal-gateway.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable signal-gateway.service
   sudo systemctl start signal-gateway.service
   ```

## 📊 Current Status

| Service | Version | Status | Uptime |
|---------|---------|--------|--------|
| Brain Server | v0.8.0 | ✅ Active | Stable |
| Signal Gateway | v0.1.0 | ✅ Active | Production-ready |
| OpenClaw | v2026.2.22-2 | ✅ Active | Stable |

## 🔧 Development

### Running Tests

```bash
# Test brain-server
cd services/brain-server && cargo test

# Test signal-gateway
cd services/signal-gateway && cargo test
```

### Building for Release

```bash
# Brain Server (ARM64 Cortex-A57 optimized)
cd services/brain-server
RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat" cargo build --release -j 1

# Signal Gateway
cd services/signal-gateway
cargo build --release
```

## 📝 Configuration

### Brain Server
- **Config:** `services/brain-server/config/`
- **Database:** `~/.brain-server/brain.db`
- **Port:** 8765

### Signal Gateway
- **Config:** `/etc/signal-gateway/config.yaml`
- **Port:** 8080
- **Wrapper:** `/usr/local/bin/signal-gateway-wrapper.sh`

### OpenClaw
- **Config:** `services/openclaw-config/config.yaml`
- **Gateway Port:** 18789

## 🛡️ Security

- ✅ All services bind to loopback (127.0.0.1)
- ✅ No internet exposure
- ✅ Fail2Ban enabled for SSH
- ✅ Systemd hardening (NoNewPrivileges, ProtectSystem, etc.)

## 📈 Performance

### Brain Server
- **Memory:** 25% (1,043MB / 4,156MB)
- **Query speed:** <1ms per search
- **Database size:** ~10MB (compressed)

### Signal Gateway
- **Memory:** ~10MB
- **Startup time:** ~2 seconds
- **Shutdown time:** 83ms
- **Retry logic:** 5 attempts with exponential backoff

## 🤝 Contributing

This is a personal repository for Mark's AI infrastructure. For questions or collaborations, please open an issue.

## 📄 License

Private repository - All rights reserved

## 👤 Author

**Mark** - [GitHub](https://github.com/[USERNAME])

---

**Built with ❤️ and ☕ on Jetson Nano**
