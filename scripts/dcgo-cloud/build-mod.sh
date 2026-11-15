#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
GAME="$DCGO_ROOT/client/DCGO_Standalone/Game"
[[ -f "$GAME/MelonLoader/Il2CppAssemblies/Assembly-CSharp.dll" ]] || {
  echo 'Launch MelonLoader once to generate game interop assemblies, then stop Wine before building.' >&2
  exit 1
}
# Windows DOTNET_ROOT is for Wine only; the build uses a native Linux SDK.
unset DOTNET_ROOT
exec "${DCGO_DOTNET:-dotnet}" build "$DCGO_SCRIPT_DIR/../../code/dcgo-melon-recorder" \
  -c Release "-p:GamePath=$GAME" "$@"
