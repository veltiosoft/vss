#!/bin/sh
# Copyright 2023 TSURUTA Takumi. All rights reserved. MIT license.
# This script uses most of the script from `vss_install` with a few tweaks for vss.
# The original project is here: https://github.com/denoland/deno_install
# 
# Copyright 2019 the Deno authors. All rights reserved. MIT license.
# TODO(everyone): Keep this script simple and easily auditable.

set -e

vss_install="${VSS_INSTALL:-$HOME/.vss}"
bin_dir="$vss_install/bin"
exe="$bin_dir/vss"

if [ ! -d "$bin_dir" ]; then
	mkdir -p "$bin_dir"
fi

if [ "$OS" = "Windows_NT" ]; then
	target="windows_amd64"
	if ! command -v unzip >/dev/null; then
		echo "Error: unzip is required to install vss.exe (see: https://github.com/veltiosoft/vss_install#unzip-is-required)." 1>&2
		exit 1
	fi
	if [ $# -eq 0 ]; then
		vss_uri="https://github.com/veltiosoft/go-vss/releases/latest/download/vss_${target}.zip"
	else
		vss_uri="https://github.com/veltiosoft/go-vss/releases/download/${1}/vss_${target}.zip"
	fi
else
	case $(uname -sm) in
	"Darwin x86_64")
        target="darwin_amd64"
        ext="zip"
        ;;
	"Darwin arm64")
        target="darwin_arm64"
        ext="zip"
        ;;
	"Linux x86_64")
        target="linux_amd64"
        ext="tar.gz"
        ;;
	"Linux aarch64")
        target="linux_arm64"
        ext="tar.gz"
        ;;
	*) target="linux_amd64" ;;
	esac
	if [ $# -eq 0 ]; then
		vss_uri="https://github.com/veltiosoft/go-vss/releases/latest/download/vss_${target}.${ext}"
	else
		vss_uri="https://github.com/veltiosoft/go-vss/releases/download/${1}/vss_${target}.${ext}"
	fi
fi

if [ ext = "zip" ]; then
	curl --fail --location --progress-bar --output "$exe.zip" "$vss_uri"
	unzip -d "$bin_dir" -o "$exe.zip"
	cp "${exe}_${target}/vss" "$bin_dir"
	chmod +x "$exe"
	rm -rf "$exe.zip" "${exe}_${target}"
else
	curl --fail --location --progress-bar --output "$exe.tar.gz" "$vss_uri"
	tar -xvf "$exe.tar.gz" -C "$bin_dir"
	cp "${exe}_${target}/vss" "$bin_dir"
	chmod +x "$exe"
	rm -rf "$exe.tar.gz" "${exe}_${target}"
fi


echo "vss was installed successfully to $exe"
if command -v vss >/dev/null; then
	echo "Run 'vss --help' to get started"
else
	case $SHELL in
	/bin/zsh) shell_profile=".zshrc" ;;
	*) shell_profile=".bashrc" ;;
	esac
	echo "Manually add the directory to your \$HOME/$shell_profile (or similar)"
	echo "  export VSS_INSTALL=\"$vss_install\""
	echo "  export PATH=\"\$VSS_INSTALL/bin:\$PATH\""
	echo "Run '$exe help' to get started"
fi