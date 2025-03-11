#!/bin/bash
# build-releases.sh

set -e

VERSION=$(grep '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
echo "Building DualMind v$VERSION"

# Create release directory
mkdir -p releases

# Build for macOS
echo "Building for macOS..."
cargo build --release

# Copy the binary to the releases directory with architecture suffix
if [[ $(uname -m) == "arm64" ]]; then
    cp target/release/dualmind releases/dualmind-$VERSION-arm64-macos
else
    cp target/release/dualmind releases/dualmind-$VERSION-x86_64-macos
fi

# Copy example files
cp .env.example releases/
cp README.md releases/

# Create a zip archive
cd releases
zip -r dualmind-$VERSION-macos.zip dualmind-$VERSION-*-macos .env.example README.md

echo "Release packages created in ./releases directory" 