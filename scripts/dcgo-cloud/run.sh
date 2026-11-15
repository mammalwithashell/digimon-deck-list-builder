#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
mkdir -p "$DCGO_ROOT/evidence"
xdpyinfo -display "$DISPLAY" >/dev/null 2>&1 || {
  echo "Start scripts/dcgo-cloud/display.sh in another terminal first" >&2; exit 1;
}
cd "$DCGO_ROOT/client/DCGO_Standalone/Game"
exec "$DCGO_WINE" DCGO.exe --melonloader.hideconsole --melonloader.disablestartscreen \
  -screen-fullscreen 0 -screen-width 1280 -screen-height 720 \
  -logFile "Z:$DCGO_ROOT/evidence/Player.log" "$@" > "$DCGO_ROOT/evidence/wine.log" 2>&1
