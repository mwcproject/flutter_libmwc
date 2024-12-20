#!/usr/bin/env bash

mkdir -p build
echo ''$(git log -1 --pretty=format:"%H")' '$(date) >> build/git_commit_version.txt

VERSIONS_FILE=../../lib/git_versions.dart
EXAMPLE_VERSIONS_FILE=../../lib/git_versions_example.dart

if [ ! -f "$VERSIONS_FILE" ]; then
    cp $EXAMPLE_VERSIONS_FILE $VERSIONS_FILE
fi

COMMIT=$(git log -1 --pretty=format:"%H")
OSX="OSX"
sed -i '' "/\/\*${OS}_VERSION/c\\
/*${OS}_VERSION*/ const ${OS}_VERSION = \"${COMMIT}\";" "$VERSIONS_FILE"

rsync -av --exclude='target' ../../rust/ build/rust/

cd build/rust

rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim

cargo build --release --target x86_64-apple-ios

cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim

outputs_dir=../../../../ios/Frameworks/MwcWallet.framework
mkdir -p ${outputs_dir}

lipo -create \
  target/x86_64-apple-ios/release/libmwc_wallet.dylib \
  target/aarch64-apple-ios-sim/release/libmwc_wallet.dylib \
  -output ${outputs_dir}/MwcWallet
  #target/aarch64-apple-ios/release/libmwc_wallet.dylib \
  
file ${outputs_dir}/MwcWallet
