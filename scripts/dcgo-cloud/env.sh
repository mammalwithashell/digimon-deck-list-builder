#!/usr/bin/env bash
# Source this file; all large/downloaded artifacts stay outside the repository.
DCGO_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export DCGO_ROOT="${DCGO_ROOT:-/workspace/dcgo-cloud}"
export DISPLAY="${DISPLAY:-:100}"
export WINEPREFIX="${WINEPREFIX:-$DCGO_ROOT/prefix3}"
export WINEARCH=win64 WINEDEBUG="${WINEDEBUG:--all}"
export LIBGL_ALWAYS_SOFTWARE=1
export LD_LIBRARY_PATH="$DCGO_ROOT/runtime/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PATH="$DCGO_ROOT/runtime/usr/bin:$PATH"
export DCGO_WINE="$DCGO_ROOT/runtime/usr/lib/wine/wine64"
export VK_DRIVER_FILES="$DCGO_ROOT/runtime/usr/share/vulkan/icd.d/lvp_icd.json"
export DXVK_CONFIG_FILE="$DCGO_SCRIPT_DIR/dxvk.conf"
export WINEDLLOVERRIDES='version=n,b;dxgi,d3d11=n,b'
export DOTNET_ROOT='C:\dotnet'
