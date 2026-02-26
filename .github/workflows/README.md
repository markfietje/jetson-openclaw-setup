# GitHub Actions Workflows

Enterprise-grade CI/CD pipeline for the Jetson AI infrastructure.

## 🍎 macOS Runner Setup with Apple Containers

**Quick Setup Guide:** Use macOS Tahoe's built-in container framework as a Docker drop-in replacement for ultra-fast CI/CD.

### Setup Steps

1. **Create Docker Symlink** (one-time setup):
   ```bash
   sudo ln -s $(which container) /usr/local/bin/docker
   ```

2. **Initialize Container System**:
   ```bash
   container system start
   ```

3. **Configure GitHub Runner** (as user, NOT root):
   ```bash
   cd ~/actions-runner
   ./config.sh --url https://github.com/YOUR_ORG/YOUR_REPO --token YOUR_TOKEN --labels apple-container,macos
   ./svc.sh install  # NO sudo!
   ./svc.sh start
   ```

4. **Auto-Start on Boot** (optional):
   ```bash
   echo "container system start 2>/dev/null || true" >> ~/.zshrc
   ```

### Why Apple Containers?

- ⚡ **Sub-second startup** - Native macOS Virtualization framework
- 💾 **70% less memory** - No Docker Desktop overhead
- 🎯 **Drop-in replacement** - Identical Docker CLI syntax
- 🔒 **Better security** - User-level isolation
- 🚀 **Native ARM64 + AMD64** - Rosetta for x86_64 containers

### Usage in Workflows

```yaml
jobs:
  build:
    runs-on: [self-hosted, apple-container]
    steps:
      - uses: actions/checkout@v4
      
      - name: Build with Docker (actually Apple Containers!)
        run: |
          docker build -t my-app .
          docker run --rm my-app npm test
```

**📖 Full Documentation:** See [docs/macos-runner-setup.md](../../docs/macos-runner-setup.md) for detailed setup instructions, troubleshooting, and examples.

**🔧 Automated Setup:** Run `./scripts/setup-macos-runner.sh` to automatically configure your macOS runner.

---

## 🔄 Workflows

### CI/Testing

#### brain-server-ci.yml
- **Triggers**: Push/PR to main/dev, changes in brain-server or workflow
- **Checks**:
  - Code formatting (rustfmt)
  - Linting (clippy with 0
  -  warnings)
  - Unit tests with coverage reporting (tarpaulin)
  - Security audit (cargo-audit) - **FAILS on critical/high vulnerabilities**
  - Documentation completeness - **FAILS on missing docs**
  - x86_64 and ARM64 builds
  - Performance benchmarks (criterion)

**Features:**
- ✅ Code coverage with 80% threshold
- ✅ Uploads coverage to Codecov
- ✅ Generates PR comments with coverage
- ✅ Deploys docs to GitHub Pages
- ✅ CI summary report

#### signal-gateway-ci.yml
- **Triggers**: Push/PR to main/dev, changes in signal-gateway or workflow
- **Checks**: Same as brain-server CI

**Features:**
- ✅ Code coverage with 80% threshold
- ✅ Uploads coverage to Codecov
- ✅ Generates PR comments with coverage
- ✅ Deploys docs to GitHub Pages
- ✅ CI summary report

#### code-quality.yml
- **Triggers**: Push/PR to main/dev
- **Checks**:
  - Rust code quality (format, lint, docs)
  - YAML validation (yamllint)
  - TOML validation (cargo check)
  - JSON validation (jq)
  - Systemd service files validation
  - Systemd timer files validation
  - Shell script linting (shellcheck)
  - Markdown linting (markdownlint)
  - Security best practices (secret scanning)
  - License check

**Features:**
- ✅ Checks for undocumented public items
- ✅ Validates all config files
- ✅ Scans for potential secrets
- ✅ Verifies .gitignore coverage
- ✅ Quality summary report

### Deployment

#### auto-deploy.yml ⭐ **Recommended**
- **Triggers**: Push to main (paths: services, scripts, configs)
- **Manual Inputs**:
  - `deploy_brain_server` (default: true)
  - `deploy_signal_gateway` (default: true)
  - `skip_tests` (default: false) - **USE WITH CAUTION**
  - `rollback_enabled` (default: true)
  - `notify_on` (always/failure/success/never, default: failure)

**Process:**
1. ✅ **Pre-deployment tests** (optional)
2. ✅ **Backup current deployment** (auto-rollback enabled)
3. ✅ **Build ARM64 binaries** (optimized for Jetson)
4. ✅ **Deploy to Jetson** (SSH with atomic swaps)
5. ✅ **Health checks** (fails deployment if unhealthy)
6. ✅ **Load testing** (10 concurrent requests)
7. ✅ **Resource monitoring** (CPU, memory, disk)
8. ✅ **Log verification** (check for errors)
9. ✅ **Automatic rollback** on failure
10. ✅ **Deployment report** (comprehensive summary)
11. ✅ **Notifications** (configurable)

**Safety Features:**
- ✅ Atomic binary swaps (no downtime)
- ✅ Automatic backups before deployment
- ✅ Health check validation (fails deployment)
- ✅ Automatic rollback on failure
- ✅ Comprehensive verification
- ✅ Notification system

**NOT recommended:** deploy.yml (legacy, use auto-deploy instead)

### Release

#### release.yml
- **Triggers**: Tag push (e.g., v1.0.0)
- **Process:**
  1. ✅ **Validate tag format** (semantic versioning)
  2. ✅ **Parse version** (v1.0.0 → 1.0.0)
  3. ✅ **Pre-release tests** (format, lint, tests, builds)
  4. ✅ **Generate changelog** (categorized commits)
  5. ✅ **Build release artifacts** (optimized ARM64 binaries)
  6. ✅ **Create GitHub release** with checksums
  7. ✅ **Deploy to Jetson** (automatic after release)
  8. ✅ **Verify deployment** (health checks)
  9. ✅ **Rollback on failure**
  10. ✅ **Release notification**

**Features:**
- ✅ Semantic versioning enforcement
- ✅ Automatic changelog generation
- ✅ Pre-built ARM64 binaries in release
- ✅ SHA256 checksums for verification
- ✅ Automatic Jetson deployment
- ✅ Rollback capability
- ✅ Comprehensive release summary

**To create release:**
```bash
# Tag release
git tag -a v1.0.0 -m "Release v1.0.0: Stable release"
git push origin v1.0.0
```

### Maintenance

#### dependencies.yml
- **Triggers**: Every Monday 9:00 AM UTC, manual
- **Process:**
  1. ✅ **Check outdated dependencies** (cargo-outdated)
  2. ✅ **Security audit** (cargo-audit) - **FAILS on critical/high**
  3. ✅ **Create update PR** (auto-created if vulnerabilities found)
  4. ✅ **Dependency report** (comprehensive summary)

**Features:**
- ✅ Automatic PR creation for security fixes
- ✅ Proper severity handling (fails on critical/high)
- ✅ Manual trigger options:
  - `create_pr` (force PR creation)
  - `force_update` (update all dependencies)
- ✅ Detailed vulnerability reporting
- ✅ Actionable recommendations

**Manual trigger:**
```bash
# Trigger via GitHub Actions UI
# Or create PR for updates:
gh workflow run dependencies.yml -f create_pr=true
```

## 📦 Artifacts

All workflows upload artifacts with extended retention:

| Artifact | Retention | Purpose |
|----------|-----------|---------|
| CI coverage reports | 30 days | Code quality analysis |
| Security audits | 30 days | Vulnerability tracking |
| Release binaries | 90 days | Production deployments |
| Benchmarks | 30 days | Performance monitoring |
| Changelogs | 90 days | Release documentation |

Download from: **Actions → Select workflow run → Artifacts section**

## 🧪 Local Testing

Before pushing, test locally:

```bash
# Format check
cd services/brain-server && cargo fmt -- --check
cd services/signal-gateway && cargo fmt -- --check

# Lint
cd services/brain-server && cargo clippy -- -D warnings
cd services/signal-gateway && cargo clippy -- -D warnings

# Test
cd services/brain-server && cargo test --verbose
cd services/signal-gateway && cargo test --verbose

# Coverage
cd services/brain-server && cargo install cargo-tarpaulin
cd services/brain-server && cargo tarpaulin --out Html

# Security audit
cd services/brain-server && cargo audit
cd services/signal-gateway && cargo audit

# Build for Jetson
cd services/brain-server && cargo build --release --target aarch64-unknown-linux-gnu
cd services/signal-gateway && cargo build --release --target aarch64-unknown-linux-gnu

# Config validation
yamllint services/openclaw-config/config.yaml
shellcheck scripts/*.sh
markdownlint ./*.md
```

## 🚀 Deployment Options

### Option 1: Automatic Deployment ⭐ (Recommended)
```bash
# Just push to main
git commit -m "feat: New feature"
git push origin main

# GitHub Actions automatically:
# - Builds ARM64 binaries
# - Deploys to Jetson
# - Restarts services
# - Runs health checks
# - Reports status
```

### Option 2: Manual Deployment
```bash
# From MacBook
cd ~/Sites/jetson-openclaw-setup/
git push origin main

# From Jetson (pull changes)
cd ~/openclaw-repo/
git pull origin main

# Services auto-restart via auto-deploy workflow
```

### Option 3: Release Deployment
```bash
# Create and push tag
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# Release workflow automatically:
# - Runs all tests
# - Creates GitHub release
# - Deploys to Jetson
# - Verifies deployment
```

## 📊 Status Badges

Add to README.md:

```markdown
![Brain Server CI](https://github.com/markfietje/jetson-openclaw-setup/workflows/Brain%20Server%20CI%20(Enterprise-Grade)/badge.svg)
![Signal Gateway CI](https://github.com/markfietje/jetson-openclaw-setup/workflows/Signal%20Gateway%20CI%20(Enterprise-Grade)/badge.svg)
![Code Quality](https://github.com/markfietje/jetson-openclaw-setup/workflows/Code%20Quality%20(Enterprise-Grade)/badge.svg)
![Auto Deploy](https://github.com/markfietje/jetson-openclaw-setup/workflows/Auto%20Deploy%20to%20Jetson%20(Enterprise-Grade)/badge.svg)
```

## 🛡️ Security Features

### Automated Security
- ✅ **Security audits** fail on critical/high vulnerabilities
- ✅ **Secret scanning** prevents committing credentials
- ✅ **Dependency monitoring** with automatic PRs
- ✅ **IP restrictions** on SSH access
- ✅ **No passwords** (SSH keys only)

### Pre-deployment Safety
- ✅ **Comprehensive testing** before deployment
- ✅ **Atomic deployments** (no partial failures)
- ✅ **Automatic backups** before changes
- ✅ **Health check validation** (fails if unhealthy)
- ✅ **Automatic rollback** on failure
- ✅ **Load testing** before verification
- ✅ **Log monitoring** for errors

### Infrastructure Security
- ✅ **IP-restricted SSH** (only MacBook can connect)
- ✅ **SSH key authentication** (no passwords)
- ✅ **Fail2ban** running (brute-force protection)
- ✅ **No secrets in code** (environment variables)
- ✅ **Protected main branch** (PR reviews required)

## 📈 Performance Monitoring

### Benchmarks
- ✅ Automated performance benchmarks (criterion)
- ✅ 30-day retention of benchmark results
- ✅ Run on every main branch push

### Metrics
- ✅ Build time tracking
- ✅ Deployment duration monitoring
- ✅ Health check response times
- ✅ Resource usage tracking (CPU, memory, disk)

## 🔔 Notifications

### Configurable Notifications
All workflows support configurable notifications:

- **Always**: Notify on every run
- **Failure only**: Only on failures (default)
- **Success only**: Only on success
- **Never**: Disable notifications

**Example:**
```yaml
# In auto-deploy.yml, set via manual trigger
notify_on: always  # Options: always, failure, success, never
```

### Notification Channels
- ✅ GitHub Actions summary reports
- ✅ Job status badges
- ✅ Artifact uploads
- ✅ PR comments (coverage, security)
- (Slack/Email can be easily added)

## 🎯 Workflow Status

| Workflow | Status | Purpose |
|----------|--------|---------|
| auto-deploy.yml | ✅ Active | Main deployment pipeline |
| brain-server-ci.yml | ✅ Active | Brain server CI |
| signal-gateway-ci.yml | ✅ Active | Signal gateway CI |
| code-quality.yml | ✅ Active | Quality gates |
| dependencies.yml | ✅ Active | Dependency management |
| release.yml | ✅ Active | Release automation |
| deploy.yml | ❌ Removed | Replaced by auto-deploy.yml |

## 📚 Quick Reference

### Common Workflows

**Deploy code:**
```bash
git push origin main  # Auto-deploys to Jetson
```

**Create release:**
```bash
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0  # Auto-deploys release to Jetson
```

**Update dependencies:**
```bash
# GitHub Actions creates PR automatically on Monday
# Or trigger manually:
gh workflow run dependencies.yml -f create_pr=true
```

### Troubleshooting

**Deployment failed:**
```bash
# Check workflow logs
gh run list --workflow=auto-deploy.yml
gh run view [run-id]

# Check service status
ssh jetson@jetson "sudo systemctl status brain-server"
ssh jetson@jetson "sudo systemctl status signal-gateway"

# Check logs
ssh jetson@jetson "sudo journalctl -u brain-server -f"
ssh jetson@jetson "sudo journalctl -u signal-gateway -f"
```

**Rollback deployment:**
```bash
# Automatic rollback already triggered if failed
# Manual rollback:
ssh jetson@jetson "sudo systemctl rollback brain-server"
```

**Security audit failed:**
```bash
# Check vulnerabilities
cd services/brain-server && cargo audit
cd services/signal-gateway && cargo audit

# Update dependencies
cd services/brain-server && cargo update
cd services/signal-gateway && cargo update
```

## 🎓 Best Practices

### Before Pushing
1. ✅ Run tests locally (`cargo test`)
2. ✅ Check formatting (`cargo fmt --check`)
3. ✅ Run linter (`cargo clippy`)
4. ✅ Verify no secrets committed
5. ✅ Check documentation completeness

### Commit Messages
Follow [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Test additions/changes
- `chore:` - Maintenance tasks

### Branch Strategy
- **main**: Production-ready code (protected)
- **feature/*** - Feature branches
- **fix/*** - Bugfix branches
- **hotfix/*** - Urgent production fixes

### Code Review
- ✅ All changes require PR (or direct push on main)
- ✅ CI must pass before merge
- ✅ Coverage must be >80%
- ✅ No security vulnerabilities
- ✅ Documentation complete

## 🚀 Advanced Usage

### Manual Deployment with Options
```bash
# Skip tests (use caution)
gh workflow run auto-deploy.yml -f skip_tests=true

# Deploy only brain-server
gh workflow run auto-deploy.yml -f deploy_signal_gateway=false

# Enable rollback
gh workflow run auto-deploy.yml -f rollback_enabled=true

# Always notify
gh workflow run auto-deploy.yml -f notify_on=always
```

### Dependency Management
```bash
# Force update all dependencies
gh workflow run dependencies.yml -f force_update=true

# Create PR for updates
gh workflow run dependencies.yml -f create_pr=true
```

### Custom Releases
```bash
# Create pre-release
git tag -a v1.0.0-beta -m "Beta release"
git push origin v1.0.0-beta

# Create release from branch
git checkout -b release/v1.0.0
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

## 📖 Documentation

- **Service Docs**: [services/brain-server/README.md](../../services/brain-server/README.md)
- **API Docs**: Deployed to GitHub Pages on main branch push
- **Deployment**: See [docs/CODING_FACTORY.md](../../docs/CODING_FACTORY.md)
- **Troubleshooting**: Check workflow logs and service logs

## 🎉 Summary

This CI/CD pipeline provides:

- ✅ **Enterprise-grade quality** (coverage, security, documentation)
- ✅ **Automatic deployments** with rollback protection
- ✅ **Zero-downtime deployments** (atomic swaps)
- ✅ **Comprehensive monitoring** (health, logs, metrics)
- ✅ **Security-first** (audits, scanning, IP restrictions)
- ✅ **Developer-friendly** (local testing, clear feedback)
- ✅ **Production-ready** (tested, verified, monitored)

Your code is automatically tested, built, deployed, and verified on every push! 🚀
