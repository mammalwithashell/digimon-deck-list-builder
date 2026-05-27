#!/usr/bin/env bash
#
# sync_cloud_runs.sh — mirror a cloud training host's runs/ directory into
# the local repo's runs/ directory so digimon-training-mcp can query cloud
# runs alongside local runs (via list_runs, run_metric, run_summary, etc.).
#
# Cloud-side job configs SHOULD set output.name with a `cloud_` prefix so
# mirrored runs are distinguishable from local runs in the merged tree.
#
# Usage:
#   scripts/sync_cloud_runs.sh <train-host>
#
# Path A — RunPod (GPU runs):
#   DIGIMON_REMOTE_PORT=<port> DIGIMON_REMOTE_RUNS=/workspace/runs/ \
#     scripts/sync_cloud_runs.sh root@<pod>.proxy.runpod.net
#
# Path B — Hetzner/DO over Tailscale (CPU runs):
#   scripts/sync_cloud_runs.sh digimon-train
#
# Recommended cron snippet on your laptop, active during cloud runs:
#   */5 * * * * cd ~/digimon && scripts/sync_cloud_runs.sh <host> \
#               >> /tmp/digimon-sync.log 2>&1
#
# Env overrides:
#   DIGIMON_REMOTE_RUNS   default ~/digimon-training/runs/
#                         RunPod default volume mount: /workspace/runs/
#   DIGIMON_LOCAL_RUNS    default ./runs/
#   DIGIMON_REMOTE_PORT   default 22 (omit -e flag if unset)
#                         RunPod: copy the port from the pod's SSH info

set -euo pipefail

HOST="${1:-}"
if [[ -z "${HOST}" ]]; then
  echo "Usage: $0 <train-host>" >&2
  echo "  See script header for RunPod vs Tailscale invocation patterns." >&2
  exit 2
fi

REMOTE_RUNS="${DIGIMON_REMOTE_RUNS:-~/digimon-training/runs/}"
LOCAL_RUNS="${DIGIMON_LOCAL_RUNS:-./runs/}"

mkdir -p "${LOCAL_RUNS}"

RSYNC_ARGS=(-az --delete-after --partial)
if [[ -n "${DIGIMON_REMOTE_PORT:-}" ]]; then
  RSYNC_ARGS+=(-e "ssh -p ${DIGIMON_REMOTE_PORT}")
fi

rsync "${RSYNC_ARGS[@]}" \
  "${HOST}:${REMOTE_RUNS}" \
  "${LOCAL_RUNS}"
