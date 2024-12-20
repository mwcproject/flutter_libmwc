#!/bin/bash

# IOS publish script WIP; doublecheck that they follow patterns in linux/android scripts before running

OS=ios
TAG_COMMIT=$(git log -1 --pretty=format:"%H")

rm -rf flutter_libmwc_bins
git clone https://git.cypherstack.com/stackwallet/flutter_libmwc_bins
if [ -d flutter_libmwc_bins ]; then
  cd flutter_libmwc_bins
else
  echo "Failed to clone flutter_libmwc_bins"
  exit 1
fi

TARGET_PATH=../build/rust/target
BIN=libmwc_wallet.a
HEADER=libmwc_wallet.h

for TARGET in aarch64-apple-ios x86_64-apple-ios
do
  if [ $(git tag -l "${TARGET}_${TAG_COMMIT}") ]; then
    echo "Tag ${TARGET}_${TAG_COMMIT} already exists!"
  else
    ARCH_PATH=$TARGET/release

    if [ -f "$TARGET_PATH/$ARCH_PATH/$BIN" ]; then
      git checkout $OS/$TARGET || git checkout -b $OS/$TARGET
      if [ ! -d $OS/$ARCH_PATH ]; then
        mkdir -p $OS/$ARCH_PATH
      fi
      cp -rf $TARGET_PATH/$ARCH_PATH/$BIN $OS/$ARCH_PATH/$BIN
      cp -rf $TARGET_PATH/../$HEADER $OS/$ARCH_PATH/$HEADER
      git add .
      git commit -m "$TARGET commit for $TAG_COMMIT"
      git push origin $OS/$TARGET
      git tag $TARGET"_$TAG_COMMIT"
      git push --tags
    else
      echo "$TARGET not found!"
    fi
  fi
done
