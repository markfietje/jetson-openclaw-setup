# GitHub Actions Workflows

Automated CI/CD for Jetson's AI infrastructure.

## 🔄 Workflows

### CI/Testing

#### brain-server-ci.yml
- Triggers: Push/PR to main/dev, changes in 
- Runs: Format check, clippy, tests, release build
- Security audit with 

#### signal-gateway-ci.yml
- Triggers: Push/PR to main/dev, changes in 
- Runs: Format check, clippy, tests, cross-compilation (ARM64)
- Security audit with 

#### code-quality.yml
- Triggers: Push/PR to main/dev
- Runs:
  - Rust formatting checks ()
  - Linting ()
  - YAML validation ()
  - Shell script checks ()
  - Systemd service file validation

### Deployment

#### deploy.yml
- Triggers: Push to main with  in commit message, or manual trigger
- Builds ARM64 binaries for Jetson
- Creates deployment packages
- Uploads artifacts (7-day retention)

**To deploy:**
```bash
git commit -m "feat: New feature [deploy]"
git push
```

Or manually trigger via GitHub Actions UI.

### Release

#### release.yml
- Triggers: Git tag push (e.g., )
- Creates GitHub release
- Auto-generates changelog from commits

**To create release:**
```bash
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

### Maintenance

#### dependencies.yml
- Triggers: Every Monday 9:00 AM UTC, or manual
- Checks for outdated dependencies ()
- Runs security audits ()
- Uploads reports as artifacts

## 📦 Artifacts

All workflows upload artifacts with 7-30 day retention:
- Binary releases (ARM64)
- Security audit reports
- Dependency reports
- Deployment packages

Download from: **Actions → Select workflow run → Artifacts section**

## 🔧 Local Testing

Before pushing, test locally:

```bash
# Format check
cd services/brain-server && cargo fmt -- --check
cd services/signal-gateway && cargo fmt -- --check

# Lint
cd services/brain-server && cargo clippy
cd services/signal-gateway && cargo clippy

# Test
cd services/brain-server && cargo test
cd services/signal-gateway && cargo test

# Build for Jetson
cd services/brain-server && cargo build --release --target aarch64-unknown-linux-gnu
cd services/signal-gateway && cargo build --release --target aarch64-unknown-linux-gnu
```

## 🚀 Deployment to Jetson

After artifacts are uploaded:

1. Download artifacts from GitHub Actions
2. Copy to Jetson: `scp artifact.tar.gz jetson:~/`
3. Extract and install

Or use the deployment script (coming soon):

```bash
./scripts/deploy-to-jetson.sh
```

## 📊 Status Badges

Add to README.md:

```markdown
![Brain Server CI](https://github.com/markfietje/jetson-openclaw-setup/workflows/Brain%20Server%20CI/badge.svg)
![Signal Gateway CI](https://github.com/markfietje/jetson-openclaw-setup/workflows/Signal%20Gateway%20CI/badge.svg)
![Code Quality](https://github.com/markfietje/jetson-openclaw-setup/workflows/Code%20Quality/badge.svg)
```
