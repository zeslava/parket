#!/bin/sh
set -e

REPO="zeslava/parket"
BIN_DIR="${HOME}/.local/bin"

detect_target() {
    OS=$(uname -s)
    ARCH=$(uname -m)

    case "$OS" in
        Linux)
            case "$ARCH" in
                x86_64) echo "x86_64-unknown-linux-gnu" ;;
                *) echo "Unsupported Linux architecture: $ARCH" >&2; exit 1 ;;
            esac
            ;;
        Darwin)
            case "$ARCH" in
                arm64) echo "aarch64-apple-darwin" ;;
                *) echo "Unsupported macOS architecture: $ARCH" >&2; exit 1 ;;
            esac
            ;;
        *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
    esac
}

TARGET=$(detect_target)

VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "$VERSION" ]; then
    echo "Failed to fetch latest release version" >&2
    exit 1
fi

ARCHIVE="parket-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "Downloading parket ${VERSION} (${TARGET})..."
curl -fsSL "$URL" -o "${TMP}/${ARCHIVE}"
tar -xzf "${TMP}/${ARCHIVE}" -C "$TMP"

mkdir -p "$BIN_DIR"
mv "${TMP}/parket" "${BIN_DIR}/parket"
chmod +x "${BIN_DIR}/parket"

echo "Installed to ${BIN_DIR}/parket"

case ":${PATH}:" in
    *":${BIN_DIR}:"*) ;;
    *)
        echo ""
        echo "Add ${BIN_DIR} to your PATH:"
        echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
        ;;
esac
