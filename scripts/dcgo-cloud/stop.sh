#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
exec "$DCGO_ROOT/runtime/usr/lib/wine/wineserver64" -k
