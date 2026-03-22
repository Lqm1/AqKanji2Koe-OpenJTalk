#!/usr/bin/env bash
set -euo pipefail

tag="$1"
bundle="aqkanji2koe-capi-${tag}-ios-xcframework"
stage="dist/${bundle}"

rm -rf "$stage"
mkdir -p "$stage"

cp -R dist/aqkanji2koe.xcframework "$stage/"
cp include/aqkanji2koe.h "$stage/"
cp README.md "$stage/"

if [[ -f LICENSE ]]; then
  cp LICENSE "$stage/"
fi

rm -f "dist/${bundle}.tar.gz"
tar -czf "dist/${bundle}.tar.gz" -C dist "${bundle}"
