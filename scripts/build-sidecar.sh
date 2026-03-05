#!/bin/bash
set -euo pipefail

# Build the desktop sidecar binary using PyInstaller.
#
# Usage:
#   ./scripts/build-sidecar.sh [gameplay|full]
#
# Profiles:
#   gameplay (default) — Greedy/random bots only, no ONNX models (~60-90MB)
#   full               — Includes ONNX runtime + bundled model weights (~90-120MB)
#
# Output:
#   src-tauri/binaries/digimon-server-<target-triple>[.exe]

PROFILE="${1:-gameplay}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

# Detect platform target triple (Tauri sidecar naming convention)
detect_target_triple() {
    local arch os_name triple

    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
    esac

    os_name="$(uname -s)"
    case "$os_name" in
        Linux)  triple="${arch}-unknown-linux-gnu" ;;
        Darwin) triple="${arch}-apple-darwin" ;;
        MINGW*|MSYS*|CYGWIN*) triple="${arch}-pc-windows-msvc" ;;
        *) echo "Unsupported OS: $os_name" >&2; exit 1 ;;
    esac

    echo "$triple"
}

TARGET_TRIPLE="$(detect_target_triple)"
echo "Building desktop sidecar (profile: $PROFILE, target: $TARGET_TRIPLE)"

# Install dependencies based on profile
if [ "$PROFILE" = "full" ]; then
    echo "Installing full dependencies..."
    pip install -r requirements-desktop.txt
    # Copy ONNX models to Tauri resources
    mkdir -p src-tauri/resources/models
    if [ -d models ] && ls models/*.onnx 1>/dev/null 2>&1; then
        cp models/*.onnx src-tauri/resources/models/
        echo "Copied ONNX models to src-tauri/resources/models/"
    else
        echo "Warning: No .onnx files found in models/ directory"
    fi
else
    echo "Installing gameplay-only dependencies..."
    pip install -r requirements-desktop.txt
fi

# Build with PyInstaller
echo "Running PyInstaller..."
pyinstaller desktop.spec --noconfirm

# Move binary to Tauri binaries directory with platform-specific name
mkdir -p src-tauri/binaries
BINARY_NAME="digimon-server"
DIST_BINARY="dist/${BINARY_NAME}"

# Add .exe extension on Windows
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        DIST_BINARY="${DIST_BINARY}.exe"
        BINARY_NAME="${BINARY_NAME}.exe"
        ;;
esac

if [ ! -f "$DIST_BINARY" ]; then
    echo "Error: PyInstaller output not found at $DIST_BINARY" >&2
    exit 1
fi

DEST="src-tauri/binaries/digimon-server-${TARGET_TRIPLE}"
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) DEST="${DEST}.exe" ;;
esac

cp "$DIST_BINARY" "$DEST"
chmod +x "$DEST"
echo "Sidecar binary: $DEST"

# Clean up PyInstaller artifacts
rm -rf build/ dist/ digimon-server.spec 2>/dev/null || true

echo "Done! Next step: cd src-tauri && cargo tauri build"
