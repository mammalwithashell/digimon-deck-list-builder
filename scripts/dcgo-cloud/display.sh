#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
mkdir -p "$DCGO_ROOT/evidence"
exec /usr/lib/xorg/Xorg "$DISPLAY" -config "$DCGO_SCRIPT_DIR/xorg.conf" -modulepath "$DCGO_ROOT/runtime/usr/lib/xorg/modules,/usr/lib/xorg/modules" -logfile "$DCGO_ROOT/evidence/Xorg.log" -nolisten tcp -noreset
