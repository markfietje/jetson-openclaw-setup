#!/bin/bash
# Build script for Brain Server v6.2 - Jetson Nano Optimized
# Reduces binary size by 50-70% and optimizes for Cortex-A57 + NEON

set -e

echo "🔨 Brain Server Build - Jetson Nano Edition"
echo "=========================================="

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're on the right architecture
ARCH=$(uname -m)
if [[ "$ARCH" != "aarch64" ]]; then
    echo -e "${YELLOW}⚠️  Warning: Not on ARM64 (current: $ARCH)${NC}"
    echo "This build script is optimized for Jetson Nano (aarch64)"
    echo "Proceeding anyway..."
fi

# Environment variables for build optimization
export CARGO_BUILD_JOBS=2
export RUSTFLAGS="-C debuginfo=0"
export CARGO_INCREMENTAL=1

echo ""
echo "📋 Build Configuration:"
echo "  - Jobs: $CARGO_BUILD_JOBS"
echo "  - Rustflags: $RUSTFLAGS"
echo "  - Target: aarch64-unknown-linux-gnu"
echo ""

# Add target if not already installed
if ! rustup target list --installed | grep -q "aarch64-unknown-linux-gnu"; then
    echo "📦 Adding aarch64-unknown-linux-gnu target..."
    rustup target add aarch64-unknown-linux-gnu
fi

# Build with Cortex-A57 optimizations
echo ""
echo "🔧 Building release binary..."
echo "  - opt-level: z (size-optimized)"
echo "  - LTO: enabled"
echo "  - Target CPU: cortex-a57"
echo "  - Features: +neon,+fp,+crc32"
echo ""

RUSTFLAGS="-C target-cpu=cortex-a57 -C target-feature=+neon,+fp,+crc32 $RUSTFLAGS" \
cargo build --release --target aarch64-unknown-linux-gnu -j 2

# Check if build succeeded
if [ ! -f "target/aarch64-unknown-linux-gnu/release/brain-server" ]; then
    echo "❌ Build failed - binary not found"
    exit 1
fi

# Get initial size
INITIAL_SIZE=$(du -h target/aarch64-unknown-linux-gnu/release/brain-server | cut -f1)
echo ""
echo "✅ Build complete!"
echo "   Initial size: $INITIAL_SIZE"

# Strip symbols (already done by Cargo.toml, but ensuring it)
echo ""
echo "✂️  Stripping symbols..."
strip --strip-all target/aarch64-unknown-linux-gnu/release/brain-server || true

STRIPPED_SIZE=$(du -h target/aarch64-unknown-linux-gnu/release/brain-server | cut -f1)
echo "   After strip: $STRIPPED_SIZE"

# UPX compression (if available)
if command -v upx &> /dev/null; then
    echo ""
    echo "🗜️  Compressing with UPX..."
    upx --best --lzma target/aarch64-unknown-linux-gnu/release/brain-server || \
    upx --best target/aarch64-unknown-linux-gnu/release/brain-server || \
    echo "   ⚠️  UPX compression failed (binary may not support it)"
    
    if [ $? -eq 0 ]; then
        FINAL_SIZE=$(du -h target/aarch64-unknown-linux-gnu/release/brain-server | cut -f1)
        echo "   Final size: $FINAL_SIZE"
    fi
else
    echo ""
    echo "⚠️  UPX not found - skipping compression"
    echo "   Install with: apt install upx (Debian/Ubuntu)"
fi

# Copy to workspace root for easy access
echo ""
echo "📋 Copying to workspace root..."
cp target/aarch64-unknown-linux-gnu/release/brain-server ../brain-server

# Show final stats
FINAL_SIZE=$(du -h ../brain-server | cut -f1)
FINAL_BYTES=$(du -b ../brain-server | cut -f1)

echo ""
echo "=========================================="
echo -e "${GREEN}✅ Build Complete!${NC}"
echo ""
echo "📊 Final Binary:"
echo "   Location: ../brain-server"
echo "   Size: $FINAL_SIZE ($FINAL_BYTES bytes)"
echo ""
echo "💡 To install as systemd service:"
echo "   systemctl --user stop brain-server"
echo "   cp ../brain-server ~/.openclaw/workspace/brain-server"
echo "   systemctl --user start brain-server"
echo ""
echo "📈 Expected Performance:"
echo "   - RAM: <150MB idle"
echo "   - Search: <5ms per query"
echo "   - Ingest: ~50-100ms per chunk"
echo ""
