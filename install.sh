#!/usr/bin/env sh
set -eu

repo="Zoranner/uupm-cli"
version="${VERSION:-latest}"
install_dir="${INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os:$arch" in
  Linux:x86_64|Linux:amd64)
    target="x86_64-unknown-linux-gnu"
    ;;
  Darwin:x86_64)
    target="x86_64-apple-darwin"
    ;;
  Darwin:arm64|Darwin:aarch64)
    target="aarch64-apple-darwin"
    ;;
  *)
    echo "Unsupported platform: $os $arch" >&2
    exit 1
    ;;
esac

asset="uupm-$target.tar.gz"
if [ "$version" = "latest" ]; then
  if command -v curl >/dev/null 2>&1; then
    resolved_version="$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
  elif command -v wget >/dev/null 2>&1; then
    resolved_version="$(wget -qO- "https://api.github.com/repos/$repo/releases/latest" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
  else
    echo "curl or wget is required." >&2
    exit 1
  fi
else
  resolved_version="$version"
fi

if [ -z "$resolved_version" ]; then
  echo "cannot resolve uupm release version." >&2
  exit 1
fi

url="https://github.com/$repo/releases/download/$resolved_version/$asset"
bin="$install_dir/uupm"

if [ ! -x "$bin" ] && command -v uupm >/dev/null 2>&1; then
  bin="$(command -v uupm)"
  install_dir="$(dirname "$bin")"
fi

if [ -x "$bin" ]; then
  current="$("$bin" --version 2>/dev/null | sed 's/^uupm /v/')"
  if [ "$current" = "$resolved_version" ]; then
    echo "uupm $resolved_version is already installed at $bin"
    exit 0
  fi
  echo "Updating uupm from $current to $resolved_version"
else
  echo "Installing uupm $resolved_version"
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

mkdir -p "$install_dir"

echo "Downloading $url"
if command -v curl >/dev/null 2>&1; then
  curl -fL "$url" -o "$tmp_dir/$asset"
elif command -v wget >/dev/null 2>&1; then
  wget -O "$tmp_dir/$asset" "$url"
else
  echo "curl or wget is required." >&2
  exit 1
fi

tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"
install -m 0755 "$tmp_dir/uupm" "$bin"

case ":$PATH:" in
  *":$install_dir:"*) ;;
  *)
    echo "$install_dir is not in PATH. Add it to your shell profile if uupm is not found."
    ;;
esac

"$bin" --version
echo "Installed uupm to $install_dir"
