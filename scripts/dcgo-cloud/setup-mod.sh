#!/usr/bin/env bash
# Run after bootstrap-debian13.sh. Does not launch the client or opt into automation.
set -euo pipefail
source "$(dirname "$0")/env.sh"
GAME="$DCGO_ROOT/client/DCGO_Standalone/Game"
[[ -f "$GAME/DCGO.exe" ]] || { echo 'Bootstrap the standalone client first' >&2; exit 1; }
mkdir -p "$DCGO_ROOT/downloads" "$WINEPREFIX/drive_c/dotnet"
if [[ ! -f "$GAME/MelonLoader/net6/MelonLoader.dll" ]]; then
  curl -fL --retry 3 https://github.com/LavaGang/MelonLoader/releases/download/v0.7.3/MelonLoader.x64.zip \
    -o "$DCGO_ROOT/downloads/MelonLoader.x64.zip"
  printf '%s  %s\n' 5b2b2f3d1cd42b59ec886c5bdc2663edae87a0097a4f4a8f58c0965a99dda416 \
    "$DCGO_ROOT/downloads/MelonLoader.x64.zip" | sha256sum -c -
  python3 -m zipfile -e "$DCGO_ROOT/downloads/MelonLoader.x64.zip" "$GAME"
fi
if [[ ! -f "$WINEPREFIX/drive_c/dotnet/shared/Microsoft.NETCore.App/6.0.36/coreclr.dll" ]]; then
  curl -fL --retry 3 https://builds.dotnet.microsoft.com/dotnet/Runtime/6.0.36/dotnet-runtime-6.0.36-win-x64.zip \
    -o "$DCGO_ROOT/downloads/dotnet-runtime-win.zip"
  printf '%s  %s\n' c077ccc342c5571ceb7a3c06d1dfbc99b989ed5a6e0f3a0b0b450531c1259615 \
    "$DCGO_ROOT/downloads/dotnet-runtime-win.zip" | sha256sum -c -
  python3 -m zipfile -e "$DCGO_ROOT/downloads/dotnet-runtime-win.zip" "$WINEPREFIX/drive_c/dotnet"
fi
printf 'Launch run.sh once, wait for Il2CppAssemblies/Assembly-CSharp.dll and the title screen, then stop.sh and build-mod.sh.\n'
