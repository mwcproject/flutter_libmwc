#!/usr/bin/env bash

# Exit immediately if any command fails
set -e

# Create a build directory
mkdir -p build
echo ''$(git log -1 --pretty=format:"%H")' '$(date) >> build/git_commit_version.txt

# Paths for Dart versioning files
VERSIONS_FILE=../../lib/git_versions.dart
EXAMPLE_VERSIONS_FILE=../../lib/git_versions_example.dart

# Copy example versions file if the main versions file doesn't exist
if [ ! -f "$VERSIONS_FILE" ]; then
    cp "$EXAMPLE_VERSIONS_FILE" "$VERSIONS_FILE"
fi

# Get the latest Git commit hash
COMMIT=$(git log -1 --pretty=format:"%H")

# Update MACOS_VERSION and clean up the line before WINDOWS_VERSION
sed -i '' -e "/\/\*MACOS_VERSION/c\\
/*MACOS_VERSION*/ const MACOS_VERSION = \"${COMMIT}\";" \
-e "/\/\*WINDOWS_VERSION/ s/^.*\/\*WINDOWS_VERSION/\/*WINDOWS_VERSION/" "$VERSIONS_FILE"

# Sync the Rust project into the build directory, excluding the 'target' folder
rsync -av --exclude='target' ../../rust/ build/rust/

# Navigate to the Rust build directory
cd build/rust

# Add required Rust targets for macOS
rustup target add x86_64-apple-darwin aarch64-apple-darwin

# Build the Rust library for each macOS target
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Create the output directory for the framework
OUTPUTS_DIR=../../../../macos/Frameworks/MwcWallet.framework
mkdir -p "$OUTPUTS_DIR"

# Combine the built libraries into a universal binary using lipo
lipo -create \
  target/x86_64-apple-darwin/release/libmwc_wallet.dylib \
  target/aarch64-apple-darwin/release/libmwc_wallet.dylib \
  -output "$OUTPUTS_DIR/MwcWallet"

# Verify the created binary
file "$OUTPUTS_DIR/MwcWallet"

# Print a success message
echo "Build and universal binary creation for macOS completed successfully!"
