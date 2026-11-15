#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
exec import -window root "${1:-$DCGO_ROOT/evidence/current.png}"
