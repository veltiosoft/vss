#!/bin/sh
# Copyright 2023 TSURUTA Takumi. All rights reserved. MIT license.
# Adapted from https://github.com/denoland/deno_install
# Copyright 2019 the Deno authors. All rights reserved. MIT license.

set -eu

if [ "$#" -gt 1 ]; then
    echo 'Usage: install.sh [VERSION]' >&2
    exit 1
fi

binary=vss
ext=tar.gz
if [ "${OS:-}" = "Windows_NT" ]; then
    target=windows_amd64
    binary=vss.exe
    ext=zip
else
    case $(uname -sm) in
        'Darwin arm64') target=darwin_arm64 ;;
        'Linux x86_64') target=linux_amd64 ;;
        'Linux aarch64') target=linux_arm64 ;;
        *) echo 'Unsupported platform. Install vss from source.' >&2; exit 1 ;;
    esac
fi

archive="vss_${target}.${ext}"
base='https://github.com/veltiosoft/vss/releases'
if [ "$#" -eq 0 ]; then
    base="$base/latest/download"
else
    # The version is a URL path segment, never a shell command or curl option.
    case "$1" in
        ''|*[!A-Za-z0-9.+-]*) echo 'Invalid release version' >&2; exit 1 ;;
    esac
    base="$base/download/$1"
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
curl --proto '=https' --proto-redir '=https' --tlsv1.2 --fail --show-error --location \
    --output "$work/$archive" "$base/$archive"
curl --proto '=https' --proto-redir '=https' --tlsv1.2 --fail --show-error --location \
    --output "$work/checksum" "$base/$archive.sha256"

# Never use a filename supplied by the downloaded checksum as a local path.
expected=$(awk -v filename="$archive" 'NF == 2 && ($2 == filename || $2 == "*" filename) { print $1 }' "$work/checksum")
if [ "${#expected}" -ne 64 ]; then
    echo 'Invalid SHA-256 checksum file' >&2
    exit 1
fi
case "$expected" in
    *[!0-9a-fA-F]*) echo 'Invalid SHA-256 checksum' >&2; exit 1 ;;
esac
if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$work/$archive")
elif command -v shasum >/dev/null 2>&1; then
    actual=$(shasum -a 256 "$work/$archive")
else
    echo 'sha256sum or shasum is required' >&2
    exit 1
fi
actual=${actual%% *}
expected=$(printf '%s' "$expected" | tr 'A-F' 'a-f')
if [ "$actual" != "$expected" ]; then
    echo 'SHA-256 checksum mismatch; vss was not installed' >&2
    exit 1
fi

# Stream only the flat executable. No archive paths are materialized on disk.
if [ "$ext" = 'zip' ]; then
    unzip -p "$work/$archive" "$binary" > "$work/binary"
else
    tar -xOf "$work/$archive" "$binary" > "$work/binary"
fi
if [ ! -s "$work/binary" ]; then
    echo 'Archive contains no executable' >&2
    exit 1
fi
chmod +x "$work/binary"
vss_install="${VSS_INSTALL:-$HOME/.vss}"
bin_dir="$vss_install/bin"
mkdir -p "$bin_dir"
mv -f "$work/binary" "$bin_dir/$binary"
printf 'vss was installed successfully to %s\n' "$bin_dir/$binary"
printf 'Add %s to PATH, then run vss --help\n' "$bin_dir"
