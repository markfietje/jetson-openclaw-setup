#!/bin/bash
# Comprehensive changelog generator for Jetson OpenClaw Setup
# Reads CHANGELOG.md and generates formatted release notes with commit details

set -euo pipefail

# Script information
SCRIPT_NAME="generate-changelog.sh"
SCRIPT_VERSION="1.0.0"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHANGELOG_FILE="${PROJECT_ROOT}/CHANGELOG.md"
OUTPUT_FILE="${PROJECT_ROOT}/build/release-notes.md"

# Version information
VERSION="${1:-}"
REPO_URL="https://github.com/markfietje/jetson-openclaw-setup"

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $*" >&2
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*" >&2
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

log_step() {
    echo -e "${CYAN}[STEP]${NC} $*" >&2
}

# Extract version from Cargo.toml files
get_latest_version() {
    # Try brain-server first
    local version=$(grep '^version = ' "${PROJECT_ROOT}/services/brain-server/Cargo.toml" | head -1 | cut -d'"' -f2)

    if [ -z "$version" ]; then
        log_error "Could not extract version from Cargo.toml"
        exit 1
    fi

    echo "$version"
}

# Get previous version from git tags
get_previous_version() {
    local current_tag="v${VERSION}"
    local prev_tag=$(git describe --tags --abbrev=0 "$current_tag^" 2>/dev/null || echo "")

    if [ -z "$prev_tag" ]; then
        echo ""
    else
        # Remove 'v' prefix
        echo "${prev_tag#v}"
    fi
}

# Extract version section from CHANGELOG.md
extract_changelog_section() {
    local version=$1
    local changelog=$2

    # Find the line number of the version header
    local start_line=$(grep -n "^\[${version}\]" "$changelog" | head -1 | cut -d: -f1)

    if [ -z "$start_line" ]; then
        log_warn "Version ${version} not found in CHANGELOG.md"
        return 1
    fi

    # Adjust to include the ## before the version
    start_line=$((start_line - 1))

    # Find the next version header (or end of relevant section)
    local end_line=$(awk "NR > ${start_line} && /^##\s+\[/ {print NR; exit}" "$changelog")

    if [ -z "$end_line" ]; then
        # No next version, read until Unreleased or end of file
        end_line=$(awk "/^##\s+\[Unreleased\]/ {print NR; exit}" "$changelog")
        if [ -z "$end_line" ]; then
            end_line=$(wc -l < "$changelog")
        fi
    fi

    end_line=$((end_line - 1))

    # Extract the section
    sed -n "${start_line},${end_line}p" "$changelog"
}

# Get commit statistics
get_commit_stats() {
    local prev_version=$1
    local current_version=$2

    local range=""
    if [ -n "$prev_version" ]; then
        range="v${prev_version}..v${current_version}"
    else
        range="v${current_version}"
    fi

    local commits=$(git rev-list $range --count 2>/dev/null || echo "0")
    local contributors=$(git shortlog -sn $range 2>/dev/null | wc -l | tr -d ' ')

    echo "- **Commits:** ${commits}"
    echo "- **Contributors:** ${contributors}"
}

# Get categorized commits
get_categorized_commits() {
    local prev_version=$1
    local current_version=$2

    local range=""
    if [ -n "$prev_version" ]; then
        range="v${prev_version}..v${current_version}"
    else
        range="v${current_version}"
    fi

    echo "### 💻 Commit History"
    echo ""

    # Features
    local features=$(git log $range --pretty=format:"- %s" 2>/dev/null | grep -iE "^- feat" || true)
    if [ -n "$features" ]; then
        echo "#### ✨ Features"
        echo "$features"
        echo ""
    fi

    # Bug fixes
    local fixes=$(git log $range --pretty=format:"- %s" 2>/dev/null | grep -iE "^- fix" || true)
    if [ -n "$fixes" ]; then
        echo "#### 🐛 Bug Fixes"
        echo "$fixes"
        echo ""
    fi

    # Improvements
    local improvements=$(git log $range --pretty=format:"- %s" 2>/dev/null | grep -iE "^- (perf|refactor|revert)" || true)
    if [ -n "$improvements" ]; then
        echo "#### ⚡ Improvements"
        echo "$improvements"
        echo ""
    fi

    # Documentation
    local docs=$(git log $range --pretty=format:"- %s" 2>/dev/null | grep -iE "^- docs" || true)
    if [ -n "$docs" ]; then
        echo "#### 📚 Documentation"
        echo "$docs"
        echo ""
    fi

    # Maintenance
    local maintenance=$(git log $range --pretty=format:"- %s" 2>/dev/null | grep -iE "^- (chore|test|ci|build)" || true)
    if [ -n "$maintenance" ]; then
        echo "#### 🔧 Maintenance"
        echo "$maintenance"
        echo ""
    fi
}

# Generate comprehensive release notes
generate_release_notes() {
    local version=$1
    local prev_version=$2
    local changelog_section=$3

    cat > "$OUTPUT_FILE" << EOF
# 🚀 Release v${version}

**Release Date:** $(date -u +"%Y-%m-%d")
**Repository:** [jetson-openclaw-setup](${REPO_URL})

---

## 📋 Release Notes

${changelog_section}

---

$(get_commit_stats "$prev_version" "$version")

---

## 📦 Installation

### Debian Package (Recommended for Jetson Nano)

Download and install the .deb packages:

\`\`\`bash
# Download packages
wget ${REPO_URL}/releases/download/v${version}/brain-server_${version}_arm64.deb
wget ${REPO_URL}/releases/download/v${version}/signal-gateway_${version}_arm64.deb

# Install packages
sudo dpkg -i brain-server_${version}_arm64.deb
sudo dpkg -i signal-gateway_${version}_arm64.deb

# Start services
sudo systemctl start brain-server signal-gateway
sudo systemctl enable brain-server signal-gateway
\`\`\`

### Binary Installation (Alternative)

\`\`\`bash
# Download binaries
wget ${REPO_URL}/releases/download/v${version}/brain-server-arm64.tar.gz
wget ${REPO_URL}/releases/download/v${version}/signal-gateway-arm64.tar.gz

# Extract and install
tar xzf brain-server-arm64.tar.gz
tar xzf signal-gateway-arm64.tar.gz
sudo mv brain-server /usr/local/bin/
sudo mv signal-gateway /usr/local/bin/
sudo chmod +x /usr/local/bin/brain-server
sudo chmod +x /usr/local/bin/signal-gateway
\`\`\`

---

## 🔐 Verification

All release artifacts include SHA256 checksums for verification.

\`\`\`bash
# Download checksums
wget ${REPO_URL}/releases/download/v${version}/SHA256SUMS.txt

# Verify packages
sha256sum -c SHA256SUMS.txt
\`\`\`

---

## 📥 Download Links

### Debian Packages (ARM64)

| Package | Size | Download |
|---------|------|----------|
| Brain Server | [\`brain-server_${version}_arm64.deb\`](${REPO_URL}/releases/download/v${version}/brain-server_${version}_arm64.deb) | Direct install with dpkg |
| Signal Gateway | [\`signal-gateway_${version}_arm64.deb\`](${REPO_URL}/releases/download/v${version}/signal-gateway_${version}_arm64.deb) | Direct install with dpkg |

### Binary Archives (ARM64)

| Service | Size | Download |
|---------|------|----------|
| Brain Server | [\`brain-server-arm64.tar.gz\`](${REPO_URL}/releases/download/v${version}/brain-server-arm64.tar.gz) | Standalone binary |
| Signal Gateway | [\`signal-gateway-arm64.tar.gz\`](${REPO_URL}/releases/download/v${version}/signal-gateway-arm64.tar.gz) | Standalone binary |

### Checksums

- [\`SHA256SUMS.txt\`](${REPO_URL}/releases/download/v${version}/SHA256SUMS.txt) - SHA256 checksums for all artifacts

---

## 🔧 Quick Start

After installation:

\`\`\`bash
# Check service status
sudo systemctl status brain-server signal-gateway

# View logs
sudo journalctl -u brain-server -f
sudo journalctl -u signal-gateway -f

# Test endpoints
curl http://localhost:8765/health
curl http://localhost:8080/v1/health
\`\`\`

---

## 📚 Documentation

- [README.md](${REPO_URL}/blob/v${version}/README.md) - Project overview and setup guide
- [API Documentation](${REPO_URL}/blob/v${version}/docs/API.md) - API endpoints and usage
- [Deployment Guide](${REPO_URL}/blob/v${version}/docs/DEPLOYMENT.md) - Deployment instructions

---

## 🔄 Upgrade from Previous Version

If upgrading from a previous version:

\`\`\`bash
# Stop services
sudo systemctl stop brain-server signal-gateway

# Upgrade packages
sudo dpkg -i brain-server_${version}_arm64.deb
sudo dpkg -i signal-gateway_${version}_arm64.deb

# Start services
sudo systemctl start brain-server signal-gateway

# Verify upgrade
sudo systemctl status brain-server signal-gateway
\`\`\`

Your configuration and data will be preserved during the upgrade.

---

## 📝 Changelog

For a complete list of changes, see the [CHANGELOG.md](${REPO_URL}/blob/v${version}/CHANGELOG.md) file.

---

$(get_categorized_commits "$prev_version" "$version")

---

**Full Changelog**: [${REPO_URL}/compare/v${prev_version}...v${version}](${REPO_URL}/compare/v${prev_version}...v${version})
EOF
}

# Main function
main() {
    # Check if CHANGELOG.md exists
    if [ ! -f "$CHANGELOG_FILE" ]; then
        log_error "CHANGELOG.md not found at ${CHANGELOG_FILE}"
        exit 1
    fi

    # Get version if not provided
    if [ -z "$VERSION" ]; then
        log_info "No version specified, extracting latest version..."
        VERSION=$(get_latest_version)
        log_info "Using version: ${VERSION}"
    fi

    # Get previous version
    PREV_VERSION=$(get_previous_version)
    if [ -n "$PREV_VERSION" ]; then
        log_info "Previous version: ${PREV_VERSION}"
    else
        log_info "No previous version found (this appears to be the first release)"
    fi

    # Extract changelog section
    log_step "Extracting changelog section for v${VERSION}..."
    CHANGELOG_SECTION=$(extract_changelog_section "$VERSION" "$CHANGELOG_FILE")

    if [ -z "$CHANGELOG_SECTION" ]; then
        log_warn "Could not extract changelog section, using fallback..."
        CHANGELOG_SECTION="See [CHANGELOG.md](${REPO_URL}/blob/v${VERSION}/CHANGELOG.md) for details."
    fi

    # Create output directory
    mkdir -p "$(dirname "$OUTPUT_FILE")"

    # Generate release notes
    log_step "Generating comprehensive release notes..."
    generate_release_notes "$VERSION" "$PREV_VERSION" "$CHANGELOG_SECTION"

    log_info "Release notes generated: ${OUTPUT_FILE}"
    log_info ""
    log_info "Preview:"
    echo "----------------------------------------"
    head -50 "$OUTPUT_FILE"
    echo "..."
    echo "----------------------------------------"
    log_info ""
    log_info "Full release notes saved to: ${OUTPUT_FILE}"
}

# Run main function
main "$@"
