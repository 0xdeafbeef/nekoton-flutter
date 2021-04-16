#!/usr/bin/env bash

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)

CORE_PATH="${SCRIPT_DIR}/core"
ANDROID_APP="${SCRIPT_DIR}/android/app"
JNI_LIBS="${ANDROID_APP}/src/main/jniLibs"

BUILD_TYPE="release"
LIBRARY_NAME="libntbindings.so"

ARCHITECTURES=(
  "arm64-v8a:aarch64-linux-android"
  "armeabi-v7a:armv7-linux-androideabi"
  "x86:i686-linux-android"
  "x86_64:x86_64-linux-android"
)

for ITEM in "${ARCHITECTURES[@]}"
do
  ARCH="${ITEM%%:*}"
  TARGET="${ITEM##*:}"

  echo "${CORE_PATH}/target/${TARGET}/${BUILD_TYPE}/${LIBRARY_NAME}"
  echo "${JNI_LIBS}/${ARCH}/${LIBRARY_NAME}"

  mkdir -p "${JNI_LIBS}/${ARCH}"
  ln -s \
    "${CORE_PATH}/target/${TARGET}/${BUILD_TYPE}/${LIBRARY_NAME}" \
    "${JNI_LIBS}/${ARCH}/${LIBRARY_NAME}"
done
