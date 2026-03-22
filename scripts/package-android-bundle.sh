#!/usr/bin/env bash
set -euo pipefail

tag="$1"
bundle="aqkanji2koe-capi-${tag}-android"
stage="dist/${bundle}"

rm -rf "$stage"
mkdir -p "${stage}/jniLibs"

cp include/aqkanji2koe.h "$stage/"
cp README.md "$stage/"

if [[ -f LICENSE ]]; then
  cp LICENSE "$stage/"
fi

for abi in armeabi-v7a arm64-v8a x86 x86_64; do
  mkdir -p "${stage}/jniLibs/${abi}"
  cp "android-jni/${abi}/libaqkanji2koe.so" "${stage}/jniLibs/${abi}/"
done

rm -f "dist/${bundle}.zip"
tar -a -cf "dist/${bundle}.zip" -C dist "${bundle}"
