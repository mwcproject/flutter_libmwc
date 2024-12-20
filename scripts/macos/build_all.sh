#!/usr/bin/env bash
mkdir build
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

#rm -rf build/rust/target/aarch64-apple-darwin/release #/libmwc_wallet.dylib
rsync -av --exclude='target' ../../rust/ build/rust/

cd build/rust

# building
#cbindgen src/lib.rs -l c > libmwc_wallet.h
#cargo lipo --release --targets aarch64-apple-darwin
cargo lipo --release #--target aarch64-apple-darwin

# moving files to the ios project
#inc=../../../../macos/include
#libs=../../../../macos/libs

outputs_dir=../../../../macos/Frameworks/MwcWallet.framework

#mkdir ${inc}
#mkdir ${libs}
cp target/aarch64-apple-darwin/release/libmwc_wallet.dylib ${outputs_dir}/MwcWallet