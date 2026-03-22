#!/usr/bin/env bash
set -euo pipefail

tag="$1"
target="$2"
platform="$3"

bundle="aqkanji2koe-capi-${tag}-${target}"
stage="dist/${bundle}"
build_dir="target/${target}/release"

rm -rf "$stage"
mkdir -p "$stage"

cp include/aqkanji2koe.h "$stage/"
cp README.md "$stage/"

if [[ -f LICENSE ]]; then
  cp LICENSE "$stage/"
fi

case "$platform" in
  windows)
    files=(
      "aqkanji2koe.dll"
      "aqkanji2koe.dll.lib"
      "aqkanji2koe.lib"
    )
    if [[ -f "${build_dir}/aqkanji2koe.pdb" ]]; then
      files+=("aqkanji2koe.pdb")
    fi
    ;;
  linux)
    files=(
      "libaqkanji2koe.so"
      "libaqkanji2koe.a"
    )
    ;;
  macos)
    files=(
      "libaqkanji2koe.dylib"
      "libaqkanji2koe.a"
    )
    ;;
  *)
    echo "unsupported platform: ${platform}" >&2
    exit 1
    ;;
esac

for file in "${files[@]}"; do
  cp "${build_dir}/${file}" "$stage/"
done

rm -f "dist/${bundle}.zip"
tar -a -cf "dist/${bundle}.zip" -C dist "${bundle}"
