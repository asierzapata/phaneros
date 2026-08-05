#!/usr/bin/env bash
# Builds phanerosd and places it at the Tauri external-bin sidecar path
# (src-tauri/binaries/phanerosd-<target-triple>) that tauri.conf.json's
# bundle.externalBin expects. Required before `cargo check`/`tauri dev`/
# `tauri build` on phaneros-desktop, since Tauri validates the sidecar
# exists at build-script time. Not committed (see .gitignore) since it's a
# build artifact that goes stale on every phaneros-daemon change.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../../.." && pwd)"
target_triple="$(rustc -Vv | awk '/^host:/ { print $2 }')"
profile="${1:-debug}"

cargo build --manifest-path "$repo_root/Cargo.toml" -p phaneros-daemon --bin phanerosd \
  $( [ "$profile" = "release" ] && echo --release )

bin_dir="$repo_root/crates/phaneros-desktop/src-tauri/binaries"
mkdir -p "$bin_dir"
cp "$repo_root/target/$profile/phanerosd" "$bin_dir/phanerosd-$target_triple"
echo "Copied phanerosd ($profile) to $bin_dir/phanerosd-$target_triple"
