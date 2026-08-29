#!/usr/bin/env bash
# Assemble an oracle-node payload from THIS (build) machine.
#
# A node needs four things and none of them is Unity:
#   1. the built player            (~492 MB) -- running it needs no licence
#   2. DCGO's C# source            (~53 MB)  -- source priority #2, for triage
#   3. general_rule.pdf + glossary (~1 MB)   -- source priority #1
#   4. the repo itself                        -- cloned separately on the node
#
# The 4.3 GB figure people remember is the Unity PROJECT, not the artifact.
# The PDFs are git-ignored by design (CLAUDE.md rule 32): they belong in the
# image, never in the repo.
set -euo pipefail

BUILD_DIR="${1:-}"
OUT="${2:-./oracle-node-payload}"

if [[ -z "$BUILD_DIR" ]]; then
    echo "usage: $0 <build-dir> [out-dir]" >&2
    echo "  e.g. $0 /d/dcgo-build/scripted-v9 /d/oracle-node-payload" >&2
    exit 2
fi
if [[ ! -f "$BUILD_DIR/manifest.json" ]]; then
    echo "no manifest.json in $BUILD_DIR -- is that a dcgo-harness build?" >&2
    exit 1
fi

# Rule 29: DCGO lives in the base repo. Never init it in a worktree.
BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
BASE_DCGO="$BASE/DCGO"
RULES="$BASE/Digimon TCG resources"

for required in "$BASE_DCGO/Assets/Scripts" "$RULES/general_rule.pdf"; do
    if [[ ! -e "$required" ]]; then
        echo "missing $required -- run this on the BUILD machine (the base repo)," >&2
        echo "not in a worktree, where DCGO is an intentionally-empty placeholder." >&2
        exit 1
    fi
done

mkdir -p "$OUT"

echo "==> player"
cp -r "$BUILD_DIR" "$OUT/player"

echo "==> DCGO C# source (scripts only -- no art, no LFS)"
mkdir -p "$OUT/dcgo-src/Assets"
cp -r "$BASE_DCGO/Assets/Scripts" "$OUT/dcgo-src/Assets/Scripts"

echo "==> rules PDFs"
mkdir -p "$OUT/rules"
cp "$RULES/general_rule.pdf" "$OUT/rules/"
cp "$RULES/glossary.pdf" "$OUT/rules/" 2>/dev/null || echo "    (glossary.pdf absent; continuing)"
# manual.pdf is 52 MB of UI reference and is deliberately NOT shipped.

cat > "$OUT/MANIFEST.txt" <<EOF
oracle-node payload
built_from : $BUILD_DIR
dcgo_commit: $(python -c "import json,sys;print(json.load(open(sys.argv[1]))['dcgo_commit'])" "$BUILD_DIR/manifest.json")
action_space_hash: $(python -c "import json,sys;print(json.load(open(sys.argv[1]))['action_space_hash'])" "$BUILD_DIR/manifest.json")
contents   : player/ dcgo-src/Assets/Scripts rules/

The action_space_hash above pins this payload to one engine revision. If
code/digimon-engine/src/action/space.rs changes, this player encodes against a
dead space and \`node up\` will refuse it: rebuild and redistribute.
EOF

echo
du -sh "$OUT"/* 2>/dev/null || true
echo
echo "payload ready: $OUT"
echo "next: copy it to the node, then see docs/runbooks/oracle-node.md"
