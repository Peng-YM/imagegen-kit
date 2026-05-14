#!/bin/bash

set -e

REPO="Peng-YM/imagegen-kit"
REPO_BRANCH="master"
BINARY_NAME="imagegen-kit"
INSTALL_DIR="${HOME}/.local/bin"
DEFAULT_VERSION="v0.3.1"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() {
    echo -e "${BLUE}INFO:${NC} $1" >&2
}

print_success() {
    echo -e "${GREEN}SUCCESS:${NC} $1" >&2
}

print_warning() {
    echo -e "${YELLOW}WARNING:${NC} $1" >&2
}

print_error() {
    echo -e "${RED}ERROR:${NC} $1" >&2
}

detect_os() {
    case "$(uname -s)" in
        Linux*) echo "linux" ;;
        Darwin*) echo "macos" ;;
        MINGW* | MSYS* | CYGWIN*) echo "windows" ;;
        *) print_error "Unsupported OS: $(uname -s)"; exit 1 ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64 | amd64) echo "x86_64" ;;
        arm64 | aarch64) echo "aarch64" ;;
        *) print_error "Unsupported architecture: $(uname -m)"; exit 1 ;;
    esac
}

download_binary() {
    local os=$1
    local arch=$2
    local tag=$3

    local extension=""
    if [ "$os" = "windows" ]; then
        extension=".exe"
    fi

    local filename="${BINARY_NAME}-${os}-${arch}${extension}"
    local url="https://github.com/${REPO}/releases/download/${tag}/${filename}"

    print_info "Downloading ${filename}..."
    print_info "URL: ${url}"

    if command -v curl >/dev/null 2>&1; then
        curl -L -o "${BINARY_NAME}${extension}" "${url}"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "${BINARY_NAME}${extension}" "${url}"
    else
        print_error "Neither curl nor wget is installed"
        return 1
    fi

    echo "${BINARY_NAME}${extension}"
}

install_binary() {
    local binary_path=$1

    mkdir -p "${INSTALL_DIR}"
    print_info "Installing to ${INSTALL_DIR}..."
    mv "${binary_path}" "${INSTALL_DIR}/"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
}

check_path() {
    if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
        print_warning "${INSTALL_DIR} is not in your PATH"
        print_info "Add this to your shell configuration:"
        echo ""
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
    fi
}

main() {
    print_info "imagegen-kit Installer"
    print_info "======================"
    echo ""

    local os
    os=$(detect_os)
    local arch
    arch=$(detect_arch)
    local tag
    tag="${1:-$DEFAULT_VERSION}"

    print_info "Detected OS: ${os}"
    print_info "Detected architecture: ${arch}"
    print_info "Version: ${tag}"

    local temp_dir
    temp_dir=$(mktemp -d)
    cd "${temp_dir}"

    local binary
    binary=$(download_binary "${os}" "${arch}" "${tag}")
    install_binary "${binary}"

    cd /
    rm -rf "${temp_dir}"

    print_success "${BINARY_NAME} installed successfully"
    print_success "Binary location: ${INSTALL_DIR}/${BINARY_NAME}"
    check_path
}

main "$@"
