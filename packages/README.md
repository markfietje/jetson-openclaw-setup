# Debian Packages for Jetson OpenClaw Setup

This directory contains the Debian packaging infrastructure for both Brain Server and Signal Gateway services. These packages make it easy to install, upgrade, and manage the services on Jetson Nano devices.

## 📦 Package Overview

### brain-server

**Package:** `brain-server_<version>_arm64.deb`

Knowledge graph and semantic search engine for Jetson AI.

**Includes:**
- Brain Server binary (`/usr/local/bin/brain-server`)
- Systemd service file (`/etc/systemd/system/brain-server.service`)
- Configuration directory (`/etc/brain-server/`)
- Data directory (`/var/lib/brain-server/`)

**Dependencies:**
- `libc6 (>= 2.31)`
- `sqlite3` (recommended)

### signal-gateway

**Package:** `signal-gateway_<version>_arm64.deb`

Lightweight Signal daemon for OpenClaw AI integration.

**Includes:**
- Signal Gateway binary (`/usr/local/bin/signal-gateway`)
- Wrapper script (`/usr/local/bin/signal-gateway-wrapper.sh`)
- Systemd service file (`/etc/systemd/system/signal-gateway.service`)
- Configuration directory (`/etc/signal-gateway/`)
- Data directory (`/var/lib/signal-gateway/`)

**Dependencies:**
- `libc6 (>= 2.31)`
- `brain-server` (recommended)

## 🏗️ Building Packages

### Prerequisites

On your build machine (macOS/Linux):

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cross-compilation tools for ARM64
# On Debian/Ubuntu:
sudo apt-get install dpkg-dev fakeroot gcc-aarch64-linux-gnu

# On macOS (with Homebrew):
brew install dpkg fakeroot
```

### Automated Build

Use the build script to build both packages automatically:

```bash
# From project root
./scripts/build-deb-packages.sh
```

This script will:
1. Build ARM64 binaries for both services
2. Create Debian package structure
3. Build .deb packages
4. Generate SHA256 checksums
5. Output packages to `build/debian-packages/`

### Manual Build

If you need to build packages manually:

```bash
# 1. Build the ARM64 binary
cd services/brain-server
cargo build --release --target aarch64-unknown-linux-gnu

# 2. Create package directory structure
cd ../../packages/brain-server
mkdir -p debian/brain-server/usr/local/bin
mkdir -p debian/brain-server/etc/systemd/system
mkdir -p debian/brain-server/etc/brain-server
mkdir -p debian/brain-server/var/lib/brain-server

# 3. Copy files
cp ../../services/brain-server/target/aarch64-unknown-linux-gnu/release/brain-server \
   debian/brain-server/usr/local/bin/
cp brain-server.service debian/brain-server/etc/systemd/system/

# 4. Build package
fakeroot dpkg-deb --build debian/brain-server
```

## 📥 Installation

### On Jetson Nano

#### From GitHub Release (Recommended)

```bash
# Download latest packages
wget https://github.com/markfietje/jetson-openclaw-setup/releases/latest/download/brain-server_0.8.1_arm64.deb
wget https://github.com/markfietje/jetson-openclaw-setup/releases/latest/download/signal-gateway_0.1.1_arm64.deb

# Install packages
sudo dpkg -i brain-server_0.8.1_arm64.deb
sudo dpkg -i signal-gateway_0.1.1_arm64.deb

# Start services
sudo systemctl start brain-server signal-gateway
sudo systemctl enable brain-server signal-gateway
```

#### From Local Build

```bash
# Copy packages to Jetson
scp build/debian-packages/*.deb jetson@jetson:/tmp/

# SSH to Jetson
ssh jetson@jetson

# Install packages
sudo dpkg -i /tmp/brain-server_*_arm64.deb
sudo dpkg -i /tmp/signal-gateway_*_arm64.deb
```

### Verify Installation

```bash
# Check package status
dpkg -l | grep -E "brain-server|signal-gateway"

# Check service status
sudo systemctl status brain-server signal-gateway

# Test endpoints
curl http://localhost:8765/health
curl http://localhost:8080/v1/health
```

## ⚙️ Configuration

### Brain Server

Configuration file: `/etc/brain-server/config.toml`

```toml
[server]
host = "127.0.0.1"
port = 8765

[database]
path = "/var/lib/brain-server/db/brain.db"

[embedding]
model = "minishlab/potion-retrieval-32M"

[logging]
level = "info"
path = "/var/lib/brain-server/logs"
```

### Signal Gateway

Configuration file: `/etc/signal-gateway/config.toml`

```toml
[server]
host = "127.0.0.1"
port = 8080

[signal]
data_dir = "/var/lib/signal-gateway/signal-data"
# phone_number = "+1234567890"  # Set this before starting

[brain_server]
url = "http://127.0.0.1:8765"
timeout_seconds = 30

[logging]
level = "info"
path = "/var/lib/signal-gateway/logs"
```

**Note:** Configuration files are marked as "conffiles" and will be preserved during package upgrades.

## 🔄 Service Management

### Start Services

```bash
sudo systemctl start brain-server
sudo systemctl start signal-gateway
```

### Stop Services

```bash
sudo systemctl stop signal-gateway
sudo systemctl stop brain-server
```

### Restart Services

```bash
sudo systemctl restart brain-server signal-gateway
```

### Enable Auto-start on Boot

```bash
sudo systemctl enable brain-server signal-gateway
```

### Disable Auto-start

```bash
sudo systemctl disable brain-server signal-gateway
```

### View Logs

```bash
# Brain Server
sudo journalctl -u brain-server -f

# Signal Gateway
sudo journalctl -u signal-gateway -f

# Both services
sudo journalctl -u brain-server -u signal-gateway -f
```

## 📊 Package Structure

```
brain-server_<version>_arm64.deb
├── DEBIAN/
│   ├── control       # Package metadata
│   ├── postinst      # Post-installation script
│   ├── prerm         # Pre-removal script
│   ├── postrm        # Post-removal script
│   └── conffiles     # Configuration files to preserve
├── usr/
│   ├── local/
│   │   └── bin/
│   │       └── brain-server
│   └── share/
│       └── doc/
│           └── brain-server/
│               └── README.md
├── etc/
│   ├── systemd/
│   │   └── system/
│   │       └── brain-server.service
│   └── brain-server/
│       └── (config created on first install)
└── var/
    └── lib/
        └── brain-server/
            ├── db/
            ├── models/
            └── logs/
```

## 🔧 Post-Installation Scripts

### What They Do

The packages include post-installation scripts that:

1. **Create system users** (`brain-server`, `signal-gateway`)
2. **Create necessary directories** with proper permissions
3. **Generate default configuration** if not present
4. **Install and enable systemd services**
5. **Set proper file permissions**

### Signal Gateway Specific

For Signal Gateway, the post-install script also:

1. Checks if Signal data directory exists
2. Prompts for phone number configuration
3. Provides instructions for Signal device linking

## 🗑️ Removal

### Remove Package (Keep Config)

```bash
sudo dpkg --remove brain-server
sudo dpkg --remove signal-gateway
```

This removes the binaries but keeps:
- Configuration files in `/etc/`
- Data files in `/var/lib/`

### Purge Package (Remove Everything)

```bash
sudo dpkg --purge brain-server
sudo dpkg --purge signal-gateway
```

This removes:
- Binaries
- Configuration files
- Systemd service files

**Note:** Data directories in `/var/lib/` are preserved unless you explicitly confirm their removal during the purge process.

## 🔄 Upgrading

### Upgrade Process

```bash
# Download new version
wget https://github.com/markfietje/jetson-openclaw-setup/releases/download/v0.9.0/brain-server_0.9.0_arm64.deb

# Stop services
sudo systemctl stop brain-server signal-gateway

# Upgrade package
sudo dpkg -i brain-server_0.9.0_arm64.deb

# Start services
sudo systemctl start brain-server signal-gateway
```

**Your configuration and data will be automatically preserved during upgrades.**

### Rollback

If an upgrade fails or causes issues:

```bash
# Stop services
sudo systemctl stop brain-server

# Install previous version
sudo dpkg -i brain-server_0.8.1_arm64.deb

# Start services
sudo systemctl start brain-server
```

## 🔍 Troubleshooting

### Package Installation Fails

```bash
# Check for dependency issues
sudo apt-get install -f

# Verify package integrity
dpkg --info brain-server_*_arm64.deb

# Check package contents
dpkg --contents brain-server_*_arm64.deb
```

### Service Won't Start

```bash
# Check service status
sudo systemctl status brain-server

# View detailed logs
sudo journalctl -u brain-server -n 100

# Check if binary exists
which brain-server
ls -l /usr/local/bin/brain-server

# Check permissions
ls -ld /var/lib/brain-server
ls -ld /etc/brain-server
```

### Permission Issues

```bash
# Fix ownership
sudo chown -R brain-server:brain-server /var/lib/brain-server
sudo chown -R brain-server:brain-server /etc/brain-server

# Fix permissions
sudo chmod 750 /var/lib/brain-server
sudo chmod 755 /etc/brain-server
```

### Configuration File Conflicts

During upgrades, if you see a configuration file conflict:

```bash
# Keep your current config
sudo apt-get install -f

# Or compare configs
diff /etc/brain-server/config.toml /etc/brain-server/config.toml.dpkg-new

# Manually merge changes and remove .dpkg-new file
```

## 🔐 Security Features

### Systemd Hardening

Both service files include comprehensive security hardening:

- `NoNewPrivileges=true` - Prevents privilege escalation
- `PrivateTmp=true` - Isolated /tmp directory
- `ProtectSystem=strict` - Read-only filesystem except allowed paths
- `ProtectHome=true` - Cannot access home directories
- `IPAddressDeny=any` - Network restrictions
- `PrivateDevices=true` - No access to physical devices
- And many more...

### User Isolation

Services run as dedicated system users:
- `brain-server` user
- `signal-gateway` user

These users:
- Have no shell access
- Cannot create home directories
- Are isolated from other users

## 📝 Customizing Packages

### Modify Control File

Edit `packages/brain-server/debian/control` to change:
- Package dependencies
- Description
- Maintainer information

### Modify Post-Install Script

Edit `packages/brain-server/debian/postinst` to:
- Add custom setup steps
- Create additional directories
- Set environment variables
- Run database migrations

### Modify Systemd Service

Edit `packages/brain-server/brain-server.service` to:
- Change resource limits
- Modify restart behavior
- Add environment variables
- Adjust security settings

After modifications, rebuild the package:

```bash
./scripts/build-deb-packages.sh
```

## 📚 Additional Resources

- [Main README](../README.md) - Project overview
- [Deployment Guide](../docs/DEPLOYMENT.md) - Detailed deployment instructions
- [API Documentation](../docs/API.md) - API endpoints
- [CHANGELOG.md](../CHANGELOG.md) - Version history

## 🤝 Contributing

If you improve the packaging:

1. Test your changes on a real Jetson Nano
2. Ensure upgrade paths work correctly
3. Update this README if needed
4. Submit a pull request

## 📄 License

These packages are part of the Jetson OpenClaw Setup project and are licensed under the same terms (MIT OR Apache-2.0).

---

**Questions or Issues?**

Open an issue at: https://github.com/markfietje/jetson-openclaw-setup/issues