#!/bin/bash

export WORKDIR="$(pwd)/"build
export CACHEDIR="$(pwd)/"cache
ANDROID_NDK_API=27b

export ANDROID_NDK_SHA1="6fc476b2e57d7c01ac0c95817746b927035b9749"
export ANDROID_NDK_URL=https://dl.google.com/android/repository/android-ndk-r${ANDROID_NDK_API}-linux.zip
# Some NDK versions end in -linux.zip, some in -linux_x86_64.zip.
export ANDROID_NDK_ZIP=${CACHEDIR}/android-ndk-r${ANDROID_NDK_API}-linux.zip
export ANDROID_NDK_ROOT=${WORKDIR}/android-ndk-r${ANDROID_NDK_API}
export ANDROID_NDK_HOME=$ANDROID_NDK_ROOT
