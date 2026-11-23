#!/usr/bin/env bash
set -euo pipefail

if [[ $# == 1 && $1 == root ]]; then
    mode=root
elif [[ $# == 2 && $1 == run ]]; then
    mode=run
    build="$(realpath -e -- "$2")"
else
    echo "Usage: $0 root | run <dcgo-harness player directory>" >&2
    exit 2
fi

: "${WINEPREFIX:?Export an absolute, dedicated WINEPREFIX for the oracle}"
if [[ "$WINEPREFIX" != /* ]]; then
    echo "WINEPREFIX must be an absolute path" >&2
    exit 2
fi
mkdir -p -- "$(dirname -- "$WINEPREFIX")"
export WINEDEBUG="${WINEDEBUG:--all}"
export WINEDLLOVERRIDES="${WINEDLLOVERRIDES:-mscoree,mshtml=}"
profile="$(wine cmd.exe /d /c 'echo %USERPROFILE%')"
profile="${profile%$'\r'}"
if [[ ! "$profile" =~ ^[[:alpha:]]: ]]; then
    echo "Could not determine the Wine user profile: $profile" >&2
    exit 1
fi
root="$(winepath -u "$profile\AppData\LocalLow\DCGO\DCGO\dcgo_harness")"
if [[ "$root" != /* ]]; then
    echo "Could not translate the Wine harness root" >&2
    exit 1
fi

if [[ "$mode" == root ]]; then
    printf '%s\n' "$root"
    exit
fi

python3 - "$build/manifest.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as f:
    if json.load(f)["executable"] != "DCGO.exe":
        sys.exit("Expected the Windows DCGO.exe build produced by dcgo-harness build")
PY

repo="$(git -C "$(dirname -- "$0")" rev-parse --show-toplevel)"
cd -- "$repo"
cargo_harness=(cargo run --quiet --locked -p dcgo-harness -- --root "$root")
"${cargo_harness[@]}" enable
"${cargo_harness[@]}" node status --build "$build"

exec 9>"$root/.wine-player.lock"
flock -n 9 || { echo "A Wine oracle is already running at $root" >&2; exit 1; }
rm -f -- "$root/harness.heartbeat"
log_path="$(winepath -w "$root/dcgo-player.log")"
cd -- "$build"
exec xvfb-run -a wine ./DCGO.exe -batchmode -nographics -logFile "$log_path"
