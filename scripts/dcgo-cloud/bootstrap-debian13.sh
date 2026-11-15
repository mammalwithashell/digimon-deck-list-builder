#!/usr/bin/env bash
# Workspace-only bootstrap for Debian 13 amd64 with configured Debian sources.
set -euo pipefail
source "$(dirname "$0")/env.sh"
[[ $(uname -m) == x86_64 ]] || { echo 'Requires x86_64' >&2; exit 1; }
mkdir -p "$DCGO_ROOT"/{downloads,client,debs,runtime,evidence,apt/lists/partial,apt/cache/archives/partial}
APT=(/usr/bin/apt-get -o "Dir::State::lists=$DCGO_ROOT/apt/lists" -o "Dir::Cache=$DCGO_ROOT/apt/cache")
"${APT[@]}" update
(cd "$DCGO_ROOT/debs" && "${APT[@]}" download wine64 libwine libz-mingw-w64 xvfb xdotool libxdo3 mesa-vulkan-drivers xserver-xorg-video-dummy)
for deb in "$DCGO_ROOT"/debs/*.deb; do dpkg-deb -x "$deb" "$DCGO_ROOT/runtime"; done
# Debian's wrapper and data lookup assume a system installation.
ln -sfn wineserver64 "$DCGO_ROOT/runtime/usr/lib/wine/wineserver"
ln -sfn . "$DCGO_ROOT/runtime/usr/share/wine/wine"
if [[ ! -f "$DCGO_ROOT/downloads/DCGO_Standalone.zip" ]]; then
  curl -fL --retry 3 https://www.dcgo.online/Builds/Standalone/DCGO_Standalone.zip -o "$DCGO_ROOT/downloads/DCGO_Standalone.zip.part"
  mv "$DCGO_ROOT/downloads/DCGO_Standalone.zip.part" "$DCGO_ROOT/downloads/DCGO_Standalone.zip"
fi
sha256sum "$DCGO_ROOT/downloads/DCGO_Standalone.zip" > "$DCGO_ROOT/evidence/client-sha256.txt"
if [[ ! -f "$DCGO_ROOT/client/DCGO_Standalone/Game/DCGO.exe" ]]; then
  python3 -m zipfile -e "$DCGO_ROOT/downloads/DCGO_Standalone.zip" "$DCGO_ROOT/client"
fi
# Preseed the native DLL before Wine bootstraps the prefix.
mkdir -p "$WINEPREFIX/drive_c/windows/system32"
cp "$DCGO_ROOT/runtime/usr/x86_64-w64-mingw32/lib/zlib1.dll" "$WINEPREFIX/drive_c/windows/system32/"
if [[ ! -f "$DCGO_ROOT/downloads/dxvk-2.3.1.tar.gz" ]]; then
  curl -fL --retry 3 https://github.com/doitsujin/dxvk/releases/download/v2.3.1/dxvk-2.3.1.tar.gz -o "$DCGO_ROOT/downloads/dxvk-2.3.1.tar.gz"
fi
printf '%s  %s\n' c51416d873ced17a6a52d1fb288f887148e9ab7841f25233b6aae24853956d2c \
  "$DCGO_ROOT/downloads/dxvk-2.3.1.tar.gz" | sha256sum -c -
tar -xzf "$DCGO_ROOT/downloads/dxvk-2.3.1.tar.gz" -C "$DCGO_ROOT/runtime"
cp "$DCGO_ROOT/runtime/dxvk-2.3.1/x64/"{d3d11,dxgi}.dll "$WINEPREFIX/drive_c/windows/system32/"
"$DCGO_WINE" --version
printf 'Next: scripts/dcgo-cloud/setup-mod.sh, then display.sh and run.sh in separate terminals.\n'
