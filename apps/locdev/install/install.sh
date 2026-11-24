#!/bin/sh
# Installs the standalone LocDev CLI (locdev + locdevd) from a GitHub release,
# verifying the archive against the release's SHA256SUMS first.
#
#   curl -fsSL https://github.com/trbros/locdev/releases/latest/download/install.sh | sh
#
# Options (environment):
#   LOCDEV_VERSION      tag to install (default: the latest release), e.g. v0.8.0
#   LOCDEV_INSTALL_DIR  where the binaries go (default: ~/.local/bin)
#   LOCDEV_REPO         owner/name of the repository (default: trbros/locdev)
#   LOCDEV_BASE_URL     a mirror holding the release's files (needs LOCDEV_VERSION)
#
# No administrator rights: it only writes locdev and locdevd in the folder above.
set -eu

repo="${LOCDEV_REPO:-trbros/locdev}"
dir="${LOCDEV_INSTALL_DIR:-$HOME/.local/bin}"

fail() { printf 'locdev install: %s\n' "$1" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || fail "needs $1"; }
need curl
need tar
need uname

case "$(uname -s)" in
  Linux) os=unknown-linux-gnu ;;
  Darwin) os=apple-darwin ;;
  *) fail "unsupported system $(uname -s); on Windows use install.ps1" ;;
esac
case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  aarch64 | arm64) arch=aarch64 ;;
  *) fail "unsupported processor $(uname -m)" ;;
esac
target="$arch-$os"

tag="${LOCDEV_VERSION:-}"
if [ -z "$tag" ]; then
  tag=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" |
    sed -n 's/^ *"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)
  [ -n "$tag" ] || fail "could not find the latest release of $repo"
fi

name="locdev-$tag-$target"
base="${LOCDEV_BASE_URL:-https://github.com/$repo/releases/download/$tag}"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

printf 'Downloading %s…\n' "$name.tar.gz"
curl -fsSL -o "$tmp/$name.tar.gz" "$base/$name.tar.gz" || fail "no CLI for $target in $tag"
curl -fsSL -o "$tmp/SHA256SUMS" "$base/SHA256SUMS" || fail "the release has no SHA256SUMS"

expected=$(awk -v f="$name.tar.gz" '$2 == f || $2 == "*" f { print $1 }' "$tmp/SHA256SUMS")
[ -n "$expected" ] || fail "$name.tar.gz is not in SHA256SUMS"
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$tmp/$name.tar.gz" | awk '{print $1}')
else
  actual=$(shasum -a 256 "$tmp/$name.tar.gz" | awk '{print $1}')
fi
[ "$expected" = "$actual" ] || fail "checksum mismatch: the download is not the published file"

tar xzf "$tmp/$name.tar.gz" -C "$tmp"
mkdir -p "$dir"
for bin in locdev locdevd; do
  install -m 0755 "$tmp/$name/$bin" "$dir/$bin"
done

printf 'Installed %s in %s\n' "$("$dir/locdev" --version)" "$dir"
case ":$PATH:" in
  *":$dir:"*) ;;
  *) printf 'Add %s to your PATH (in your shell profile) to run `locdev` anywhere.\n' "$dir" ;;
esac
printf 'Next: `locdev start`, then `locdev` opens the terminal UI.\n'
