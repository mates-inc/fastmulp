#!/usr/bin/env bash
set -euo pipefail

version="$(
  awk '
    $0 == "name = \"wasm-bindgen\"" { found=1; next }
    found && /^version = "/ {
      gsub(/^version = "/, "", $0)
      gsub(/"$/, "", $0)
      print
      exit
    }
  ' Cargo.lock
)"

test -n "${version}"

installed="$(wasm-bindgen --version 2>/dev/null | awk '{ print $2 }' || true)"
if [[ "${installed}" == "${version}" ]]; then
  echo "wasm-bindgen ${version} is already installed"
  exit 0
fi

cargo install wasm-bindgen-cli --version "${version}" --locked
