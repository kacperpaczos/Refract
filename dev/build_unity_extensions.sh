#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

SRC_DIR="${UNITY_EXTENSIONS_SRC_DIR:-$ROOT_DIR/vendor/unity-shell-extensions/ubuntu-base}"
BUILD_ROOT="${UNITY_EXTENSIONS_BUILD_ROOT:-$ROOT_DIR/target/unity-shell-extensions}"
BUILD_DIR="${UNITY_EXTENSIONS_BUILD_DIR:-$BUILD_ROOT/build}"
STAGE_DIR="${UNITY_EXTENSIONS_STAGE_DIR:-$BUILD_ROOT/stage}"
PREFIX="${UNITY_EXTENSIONS_PREFIX:-/usr}"

log() {
    printf '[unity-ext] %s\n' "$1"
}

fail() {
    printf '[unity-ext] ERROR: %s\n' "$1" >&2
    exit 1
}

require_command() {
    local cmd="$1"
    command -v "$cmd" >/dev/null 2>&1 || fail "Missing required command: $cmd"
}

is_ubuntu() {
    [ -f /etc/os-release ] && {
        grep -qi '^ID=ubuntu' /etc/os-release || grep -qi '^ID_LIKE=.*ubuntu' /etc/os-release
    }
}

ensure_ubuntu_packages() {
    local missing_packages=()

    command -v git >/dev/null 2>&1 || missing_packages+=("git")
    command -v jq >/dev/null 2>&1 || missing_packages+=("jq")
    command -v make >/dev/null 2>&1 || missing_packages+=("make")
    command -v meson >/dev/null 2>&1 || missing_packages+=("meson")
    command -v ninja >/dev/null 2>&1 || missing_packages+=("ninja-build")
    command -v sassc >/dev/null 2>&1 || missing_packages+=("sassc")
    command -v msgfmt >/dev/null 2>&1 || missing_packages+=("gettext")

    if [ "${#missing_packages[@]}" -eq 0 ]; then
        return
    fi

    is_ubuntu || fail "Missing required system packages: ${missing_packages[*]}. Automatic install is supported only on Ubuntu."
    require_command pkexec
    require_command apt-get

    log "Installing missing Ubuntu packages via pkexec: ${missing_packages[*]}"
    pkexec env DEBIAN_FRONTEND=noninteractive apt-get update
    pkexec env DEBIAN_FRONTEND=noninteractive apt-get install -y "${missing_packages[@]}"

    command -v meson >/dev/null 2>&1 || fail "meson is still unavailable after apt install"
    command -v ninja >/dev/null 2>&1 || fail "ninja is still unavailable after apt install"
    command -v sassc >/dev/null 2>&1 || fail "sassc is still unavailable after apt install"
    command -v msgfmt >/dev/null 2>&1 || fail "msgfmt is still unavailable after apt install"
}

log "Preparing Unity shell extensions build"
log "Source dir: $SRC_DIR"
log "Build dir: $BUILD_DIR"
log "Stage dir: $STAGE_DIR"

[ -d "$SRC_DIR" ] || fail "Source directory does not exist: $SRC_DIR"

require_command git
require_command jq
require_command make
ensure_ubuntu_packages

mkdir -p "$BUILD_ROOT"
rm -rf "$BUILD_DIR" "$STAGE_DIR"

log "Downloading Meson subprojects declared in wrap files"
meson subprojects download --sourcedir "$SRC_DIR"

log "Configuring build directory"
meson setup "$BUILD_DIR" "$SRC_DIR" --prefix "$PREFIX"

log "Compiling extensions"
meson compile -C "$BUILD_DIR"

log "Installing into local staging directory"
meson install -C "$BUILD_DIR" --destdir "$STAGE_DIR"

log "Build finished successfully"
log "Installed files staged under: $STAGE_DIR"
