#!/bin/bash
# build-release.sh - Build release binaries for all platforms
# This script is used by GitHub Actions to create distribution packages

set -e

VERSION=${1:-$(git describe --tags --always --dirty 2>/dev/null || echo "0.1.0")}
BINARY_NAME="env-manager"

echo "🔨 Building release binaries for env-manager v${VERSION}..."
echo ""

# Function to build for a specific target
build_target() {
    local target=$1
    local artifact_name="${BINARY_NAME}-${VERSION}-${target}.tar.gz"
    
    echo "📦 Building for ${target}..."
    
    # Build the binary
    cargo build --release --target ${target} --verbose
    
    # Package the binary
    cd target/${target}/release
    tar czf ../../../${artifact_name} ${BINARY_NAME}
    cd ../../..
    
    echo "✅ Created ${artifact_name}"
    ls -lh ${artifact_name}
    echo ""
}

# Check if we're on macOS (can cross-compile for both macOS architectures)
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "🍎 Building for macOS platforms..."
    
    # macOS Apple Silicon (M1/M2/M3)
    rustup target add aarch64-apple-darwin 2>/dev/null || true
    build_target "aarch64-apple-darwin"
    
    # macOS Intel
    rustup target add x86_64-apple-darwin 2>/dev/null || true
    build_target "x86_64-apple-darwin"
    
else
    echo "🐧 Building for Linux platforms..."
    
    # Linux x86_64
    rustup target add x86_64-unknown-linux-gnu 2>/dev/null || true
    build_target "x86_64-unknown-linux-gnu"
    
    # Note: ARM64 Linux requires cross-compilation setup
    # This is typically done in CI/CD with proper toolchain
    echo "⚠️  For Linux ARM64, use GitHub Actions CI/CD"
fi

echo "✨ All builds complete!"
echo ""
echo "📊 Generated artifacts:"
ls -lh ${BINARY_NAME}-*.tar.gz
echo ""
echo "💡 Next steps:"
echo "   1. Upload these artifacts to GitHub Release"
echo "   2. Calculate SHA256: shasum -a 256 ${BINARY_NAME}-${VERSION}-aarch64-apple-darwin.tar.gz"
echo "   3. Update Homebrew formula with new version and hash"
