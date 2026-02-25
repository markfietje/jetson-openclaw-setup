#!/bin/bash
# Build Debian packages for Jetson OpenClaw Setup
# This script builds ARM64 binaries and creates .deb packages for both services

set -euo pipefail

# Script information
SCRIPT_NAME="build-deb-packages.sh"
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
BUILD_DIR="${PROJECT_ROOT}/build"
PACKAGES_DIR="${PROJECT_ROOT}/packages"
OUTPUT_DIR="${BUILD_DIR}/debian-packages"
TARGET="aarch64-unknown-linux-gnu"
LINKER="aarch64-linux-gnu-gcc"

# Version information (will be extracted from Cargo.toml)
BRAIN_SERVER_VERSION=""
SIGNAL_GATEWAY_VERSION=""

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

log_step() {
    echo -e "${CYAN}[STEP]${NC} $*"
}

log_section() {
    echo ""
    echo -e "${MAGENTA}═══════════════════════════════════════════════════════${NC}"
    echo -e "${MAGENTA}  $*${NC}"
    echo -e "${MAGENTA}═══════════════════════════════════════════════════════${NC}"
    echo ""
}

# Utility functions
check_dependencies() {
    log_step "Checking dependencies..."

    local missing_deps=()

    # Check for required tools
    command -v cargo >/dev/null 2>&1 || missing_deps+=("cargo (Rust)")
    command -v dpkg-deb >/dev/null 2>&1 || missing_deps+=("dpkg-deb")
    command -v fakeroot >/dev/null 2>&1 || missing_deps+=("fakeroot")
    command -v aarch64-linux-gnu-gcc >/dev/null 2>&1 || missing_deps+=("gcc-aarch64-linux-gnu (ARM64 cross-compiler)")
    command -v sha256sum >/dev/null 2>&1 || missing_deps+=("sha256sum")

    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies:"
        for dep in "${missing_deps[@]}"; do
            echo "  - $dep"
        done
        echo ""
        log_info "Install missing dependencies with:"
        echo "  # On Debian/Ubuntu:"
        echo "  sudo apt-get install dpkg-dev fakeroot gcc-aarch64-linux-gnu"
        echo ""
        echo "  # For Rust (if not installed):"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    log_success "All dependencies found"
}

extract_versions() {
    log_step "Extracting version information..."

    # Extract brain-server version
    BRAIN_SERVER_VERSION=$(grep '^version = ' "${PROJECT_ROOT}/services/brain-server/Cargo.toml" | head -1 | cut -d'"' -f2)
    if [ -z "$BRAIN_SERVER_VERSION" ]; then
        log_error "Could not extract brain-server version from Cargo.toml"
        exit 1
    fi
    log_info "Brain Server version: ${BRAIN_SERVER_VERSION}"

    # Extract signal-gateway version
    SIGNAL_GATEWAY_VERSION=$(grep '^version = ' "${PROJECT_ROOT}/services/signal-gateway/Cargo.toml" | head -1 | cut -d'"' -f2)
    if [ -z "$SIGNAL_GATEWAY_VERSION" ]; then
        log_error "Could not extract signal-gateway version from Cargo.toml"
        exit 1
    fi
    log_info "Signal Gateway version: ${SIGNAL_GATEWAY_VERSION}"
}

build_binaries() {
    log_section "Building ARM64 Binaries"

    # Build brain-server
    log_step "Building brain-server for ARM64..."
    cd "${PROJECT_ROOT}/services/brain-server"

    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="${LINKER}"
    cargo build --release --target "${TARGET}"

    if [ ! -f "target/${TARGET}/release/brain-server" ]; then
        log_error "Brain server binary not found after build"
        exit 1
    fi

    local brain_size=$(du -h "target/${TARGET}/release/brain-server" | cut -f1)
    log_success "Brain Server built successfully (${brain_size})"

    # Build signal-gateway
    log_step "Building signal-gateway for ARM64..."
    cd "${PROJECT_ROOT}/services/signal-gateway"

    cargo build --release --target "${TARGET}"

    if [ ! -f "target/${TARGET}/release/signal-gateway" ]; then
        log_error "Signal Gateway binary not found after build"
        exit 1
    fi

    local signal_size=$(du -h "target/${TARGET}/release/signal-gateway" | cut -f1)
    log_success "Signal Gateway built successfully (${signal_size})"

    cd "${PROJECT_ROOT}"
}

prepare_output_directory() {
    log_step "Preparing output directory..."

    rm -rf "${OUTPUT_DIR}"
    mkdir -p "${OUTPUT_DIR}"

    log_success "Output directory created: ${OUTPUT_DIR}"
}

build_deb_package() {
    local service_name=$1
    local version=$2
    local package_name="${service_name}_${version}_arm64"

    log_step "Building ${service_name} Debian package..."

    cd "${PACKAGES_DIR}/${service_name}"

    # Create package directory structure
    local pkg_dir="${OUTPUT_DIR}/${package_name}"
    rm -rf "${pkg_dir}"
    mkdir -p "${pkg_dir}/DEBIAN"

    # Copy control files
    cp debian/control "${pkg_dir}/DEBIAN/"
    cp debian/postinst "${pkg_dir}/DEBIAN/" 2>/dev/null || true
    cp debian/prerm "${pkg_dir}/DEBIAN/" 2>/dev/null || true
    cp debian/postrm "${pkg_dir}/DEBIAN/" 2>/dev/null || true

    # Make scripts executable
    chmod 755 "${pkg_dir}/DEBIAN/"* 2>/dev/null || true

    # Create directory structure
    mkdir -p "${pkg_dir}/usr/local/bin"
    mkdir -p "${pkg_dir}/etc/systemd/system"
    mkdir -p "${pkg_dir}/etc/${service_name}"
    mkdir -p "${pkg_dir}/var/lib/${service_name}"
    mkdir -p "${pkg_dir}/usr/share/doc/${service_name}"

    # Copy binary
    local binary_source="${PROJECT_ROOT}/services/${service_name}/target/${TARGET}/release/${service_name}"
    if [ -f "${binary_source}" ]; then
        cp "${binary_source}" "${pkg_dir}/usr/local/bin/"
        chmod 755 "${pkg_dir}/usr/local/bin/${service_name}"
    else
        log_error "Binary not found: ${binary_source}"
        exit 1
    fi

    # Copy systemd service file
    if [ -f "${service_name}.service" ]; then
        cp "${service_name}.service" "${pkg_dir}/etc/systemd/system/"
        chmod 644 "${pkg_dir}/etc/systemd/system/${service_name}.service"
    fi

    # Copy wrapper script for signal-gateway
    if [ "${service_name}" = "signal-gateway" ]; then
        if [ -f "${PROJECT_ROOT}/scripts/signal-gateway-wrapper.sh" ]; then
            cp "${PROJECT_ROOT}/scripts/signal-gateway-wrapper.sh" "${pkg_dir}/usr/local/bin/"
            chmod 755 "${pkg_dir}/usr/local/bin/signal-gateway-wrapper.sh"
        fi
    fi

    # Copy documentation
    if [ -f "${PROJECT_ROOT}/README.md" ]; then
        cp "${PROJECT_ROOT}/README.md" "${pkg_dir}/usr/share/doc/${service_name}/"
    fi
    if [ -f "${PROJECT_ROOT}/LICENSE" ]; then
        cp "${PROJECT_ROOT}/LICENSE" "${pkg_dir}/usr/share/doc/${service_name}/" 2>/dev/null || true
    fi

    # Create conffiles (to preserve user config)
    echo "/etc/${service_name}/config.toml" > "${pkg_dir}/DEBIAN/conffiles"

    # Build the .deb package
    cd "${OUTPUT_DIR}"
    fakeroot dpkg-deb --build "${package_name}"

    if [ ! -f "${package_name}.deb" ]; then
        log_error "Failed to build ${service_name} package"
        exit 1
    fi

    local deb_size=$(du -h "${package_name}.deb" | cut -f1)
    log_success "${service_name} package built: ${package_name}.deb (${deb_size})"

    cd "${PROJECT_ROOT}"
}

verify_packages() {
    log_section "Verifying Packages"

    cd "${OUTPUT_DIR}"

    for deb_file in *.deb; do
        if [ -f "$deb_file" ]; then
            log_step "Verifying ${deb_file}..."

            # Check package info
            if dpkg-deb --info "$deb_file" >/dev/null 2>&1; then
                log_success "Package structure valid: ${deb_file}"
            else
                log_error "Invalid package structure: ${deb_file}"
                exit 1
            fi

            # Check contents
            local num_files=$(dpkg-deb --contents "$deb_file" | wc -l)
            log_info "Contains ${num_files} files/directories"

            # Show package info
            echo ""
            dpkg-deb --info "$deb_file" | grep -E "Package|Version|Architecture|Description" || true
            echo ""
        fi
    done

    cd "${PROJECT_ROOT}"
}

generate_checksums() {
    log_section "Generating Checksums"

    cd "${OUTPUT_DIR}"

    if [ -f "SHA256SUMS.txt" ]; then
        rm "SHA256SUMS.txt"
    fi

    for deb_file in *.deb; do
        if [ -f "$deb_file" ]; then
            sha256sum "$deb_file" >> "SHA256SUMS.txt"
        fi
    done

    log_success "Checksums generated in SHA256SUMS.txt"

    # Display checksums
    echo ""
    cat "SHA256SUMS.txt"
    echo ""

    cd "${PROJECT_ROOT}"
}

print_summary() {
    log_section "Build Summary"

    echo "Package versions:"
    echo "  - Brain Server: v${BRAIN_SERVER_VERSION}"
    echo "  - Signal Gateway: v${SIGNAL_GATEWAY_VERSION}"
    echo ""
    echo "Output directory: ${OUTPUT_DIR}"
    echo ""
    echo "Generated packages:"

    cd "${OUTPUT_DIR}"

    for deb_file in *.deb; do
        if [ -f "$deb_file" ]; then
            local size=$(du -h "$deb_file" | cut -f1)
            echo "  - ${deb_file} (${size})"
        fi
    done

    echo ""
    echo "Checksums file: SHA256SUMS.txt"
    echo ""

    log_info "To install a package on Jetson:"
    echo "  sudo dpkg -i ${OUTPUT_DIR}/<package-name>.deb"
    echo ""
    log_info "To verify checksums:"
    echo "  sha256sum -c ${OUTPUT_DIR}/SHA256SUMS.txt"
    echo ""

    cd "${PROJECT_ROOT}"
}

main() {
    log_section "Jetson OpenClaw Debian Package Builder v${SCRIPT_VERSION}"

    # Print configuration
    log_info "Configuration:"
    echo "  Project root: ${PROJECT_ROOT}"
    echo "  Target: ${TARGET}"
    echo "  Linker: ${LINKER}"
    echo ""

    # Execute build steps
    check_dependencies
    extract_versions
    prepare_output_directory
    build_binaries
    build_deb_package "brain-server" "${BRAIN_SERVER_VERSION}"
    build_deb_package "signal-gateway" "${SIGNAL_GATEWAY_VERSION}"
    verify_packages
    generate_checksums
    print_summary

    log_success "All packages built successfully!"
}

# Run main function
main "$@"
