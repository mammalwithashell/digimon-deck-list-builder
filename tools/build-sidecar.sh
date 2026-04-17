#!/bin/bash
set -euo pipefail

# Build the desktop sidecar binary using PyInstaller — or skip it entirely
# and produce a Rust-engine-only bundle.
#
# Usage:
#   ./tools/build-sidecar.sh [gameplay|full|rust]
#
# Profiles:
#   gameplay (default) — Greedy/random bots only, no ONNX models (~60-90MB)
#   full               — Includes ONNX runtime + bundled model weights (~90-120MB)
#   rust               — Skip PyInstaller entirely. The frontend dispatches
#                        directly to the in-process digimon-engine crate
#                        via Tauri `invoke()`. Build the final desktop bundle
#                        with:
#                          cd src-tauri && cargo tauri build \
#                            --features no-sidecar \
#                            --config tauri.rust.conf.json
#
# Output (gameplay/full):
#   src-tauri/binaries/digimon-server-<target-triple>[.exe]

PROFILE="${1:-gameplay}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

# Rust-engine profile: no Python, no PyInstaller, no sidecar binary.
# Just prep the frontend and hand off to `cargo tauri build`.
if [ "$PROFILE" = "rust" ]; then
    echo "Building desktop (profile: rust — Rust engine, no Python sidecar)"
    export VITE_BUILD_TARGET=desktop
    export VITE_ENGINE=rust

    echo "Building frontend with Rust-engine mode..."
    (cd frontend && npm run build -- --mode rust)

    echo ""
    echo "Frontend built. Next step:"
    echo "  cd src-tauri && cargo tauri build \\"
    echo "      --features no-sidecar \\"
    echo "      --config tauri.rust.conf.json"
    echo ""
    echo "For dev iteration:"
    echo "  cd src-tauri && cargo tauri dev \\"
    echo "      --features no-sidecar \\"
    echo "      --config tauri.rust.conf.json"
    exit 0
fi

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
    echo "Installing full dependencies (including PyTorch for ONNX export)..."
    pip install -r requirements.txt

    # Export SB3 checkpoints to ONNX format
    mkdir -p models
    EXPORTED=0

    for zip in models/*.zip; do
        [ -f "$zip" ] || continue
        base="$(basename "$zip" .zip)"
        onnx_out="models/${base}.onnx"

        if [ -f "$onnx_out" ]; then
            echo "ONNX already exists: $onnx_out (skipping)"
        else
            # Detect model type from filename: *lstm* → lstm, else mlp
            if echo "$base" | grep -qi "lstm"; then
                model_type="lstm"
            else
                model_type="mlp"
            fi
            echo "Exporting $zip → $onnx_out (type: $model_type)..."
            python scripts/export_onnx.py --type "$model_type" --input "$zip" --output "$onnx_out"
            EXPORTED=$((EXPORTED + 1))
        fi
    done

    if [ "$EXPORTED" -gt 0 ]; then
        echo "Exported $EXPORTED model(s) to ONNX"
    fi

    # Copy ONNX models to Tauri resources
    mkdir -p src-tauri/resources/models
    if ls models/*.onnx 1>/dev/null 2>&1; then
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
