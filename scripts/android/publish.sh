#!/bin/bash

OS=android
TAG_COMMIT=$(git log -1 --pretty=format:"%H")

rm -rf flutter_libmwc_bins
git clone https://git.cypherstack.com/stackwallet/flutter_libmwc_bins
if [ -d flutter_libmwc_bins ]; then
  cd flutter_libmwc_bins
else
  echo "Failed to clone flutter_libmwc_bins"
  exit 1
fi

TARGET_PATH=../../../android/src/main/jniLibs
BIN=libmwc_wallet.so

for TARGET in arm64-v8a armeabi-v7a x86_64
do
  if [ $(git tag -l "${OS}_${TARGET}_${TAG_COMMIT}") ]; then
    echo "Tag ${OS}_${TARGET}_${TAG_COMMIT} already exists!"
  else
    ARCH_PATH=$TARGET

    if [ -f "$TARGET_PATH/$ARCH_PATH/$BIN" ]; then
      git checkout $OS/$TARGET || git checkout -b $OS/$TARGET
      if [ ! -d $OS/$ARCH_PATH ]; then
        mkdir -p $OS/$ARCH_PATH
      fi
      cp -rf $TARGET_PATH/$ARCH_PATH/$BIN $OS/$ARCH_PATH/$BIN
      git add .
      git commit -m "$TARGET commit for $TAG_COMMIT"
      git push origin $OS/$TARGET
      git tag "${OS}_${TARGET}_${TAG_COMMIT}"
      git push --tags
    else
      echo "$TARGET not found at $TARGET_PATH/$ARCH_PATH/$BIN!"
    fi
  fi
done
