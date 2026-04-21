#!/usr/bin/env bash
# Train a model on a DigitalOcean GPU Droplet, then pull the result back.
#
# Usage:
#   ./scripts/train_remote.sh <droplet_ip> <job_config_path> [--publish]
#
# Prerequisites on the droplet (one-time setup):
#   apt-get install -y docker.io nvidia-container-toolkit
#   systemctl restart docker
#   # Verify GPU is visible:
#   docker run --gpus all --rm nvidia/cuda:12.0-base nvidia-smi
#
# Required env vars (set in .env.training or export before running):
#   PGPASSWORD        — DigiLab PostgreSQL password (for scoped meta queries)
#   SPACES_KEY        — DigitalOcean Spaces access key   (only needed with --publish)
#   SPACES_SECRET     — DigitalOcean Spaces secret key   (only needed with --publish)
#   API_BASE_URL      — Hosted API base URL              (only needed with --publish)
#
# Optional env vars:
#   DIGILAB_CONN_STR  — Override the default DigiLab DB connection string
#   SSH_KEY           — Path to SSH private key (default: ~/.ssh/id_rsa)

set -euo pipefail

DROPLET_IP="${1:?Usage: $0 <droplet_ip> <job_config_path> [--publish]}"
JOB_CONFIG="${2:?Usage: $0 <droplet_ip> <job_config_path> [--publish]}"
PUBLISH="${3:-}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/id_rsa}"
REMOTE_DIR="/app/digimon-trainer"
REMOTE_MODELS="/mnt/models"

# Load .env.training if it exists
if [[ -f .env.training ]]; then
    set -o allexport
    # shellcheck disable=SC1091
    source .env.training
    set +o allexport
fi

# ── Validate job config exists ─────────────────────────────────────────────
if [[ ! -f "$JOB_CONFIG" ]]; then
    echo "Error: job config not found: $JOB_CONFIG" >&2
    exit 1
fi

# Extract job metadata from config
JOB_NAME=$(python3 -c "import json,sys; print(json.load(open('$JOB_CONFIG'))['output']['name'])")
MODEL_TYPE=$(python3 -c "import json,sys; c=json.load(open('$JOB_CONFIG')); print('lstm' if c.get('training',{}).get('use_lstm') else 'mlp')")

echo "================================================================"
echo "Remote training: $JOB_NAME"
echo "  Droplet:    $DROPLET_IP"
echo "  Job config: $JOB_CONFIG"
echo "  Model type: $MODEL_TYPE"
echo "================================================================"

SSH_OPTS="-i $SSH_KEY -o StrictHostKeyChecking=no -o ConnectTimeout=30"

# ── Step 1: Sync repo to droplet ──────────────────────────────────────────
echo ""
echo "[1/5] Syncing code to droplet..."
rsync -az --delete \
    -e "ssh $SSH_OPTS" \
    --exclude='.git' \
    --exclude='models/' \
    --exclude='runs/' \
    --exclude='__pycache__' \
    --exclude='*.pyc' \
    --exclude='.env*' \
    --exclude='node_modules' \
    --exclude='target' \
    . "root@${DROPLET_IP}:${REMOTE_DIR}/"

# Copy the specific job config (in case training_jobs/ was excluded or partial)
scp $SSH_OPTS "$JOB_CONFIG" "root@${DROPLET_IP}:${REMOTE_DIR}/${JOB_CONFIG}"

echo "Sync complete."

# ── Step 2: Build Docker image ────────────────────────────────────────────
echo ""
echo "[2/5] Building Docker image on droplet (cached after first build)..."
ssh $SSH_OPTS "root@${DROPLET_IP}" "
    cd ${REMOTE_DIR} && \
    docker build -f Dockerfile.training -t digimon-trainer . 2>&1
"

# ── Step 3: Run training ──────────────────────────────────────────────────
echo ""
echo "[3/5] Running training job: $JOB_NAME"
ssh $SSH_OPTS "root@${DROPLET_IP}" "
    mkdir -p ${REMOTE_MODELS} && \
    docker run --gpus all --rm \
        -v ${REMOTE_MODELS}:/app/models \
        -e DIGIMON_BACKEND=rust \
        -e DIGIMON_CARDS_JSON=/app/digimon_gym/engine/data/cards.json \
        ${PGPASSWORD:+-e PGPASSWORD='${PGPASSWORD}'} \
        ${DIGILAB_CONN_STR:+-e DIGILAB_CONN_STR='${DIGILAB_CONN_STR}'} \
        digimon-trainer ${JOB_CONFIG}
"

# ── Step 4: Pull models back ──────────────────────────────────────────────
echo ""
echo "[4/5] Pulling trained models back..."
mkdir -p models
rsync -az \
    -e "ssh $SSH_OPTS" \
    "root@${DROPLET_IP}:${REMOTE_MODELS}/" \
    ./models/

echo "Model files:"
ls -lh "models/pilot_ppo_${JOB_NAME}"* 2>/dev/null || echo "  (no files matching pilot_ppo_${JOB_NAME}*)"

# ── Step 5: Export ONNX locally ───────────────────────────────────────────
echo ""
echo "[5/5] Exporting ONNX model..."
ZIP_PATH="models/pilot_ppo_${JOB_NAME}.zip"
ONNX_PATH="models/pilot_ppo_${JOB_NAME}.onnx"

if [[ ! -f "$ZIP_PATH" ]]; then
    echo "WARNING: Model zip not found at $ZIP_PATH — skipping ONNX export."
else
    python tools/export_onnx.py \
        --type "$MODEL_TYPE" \
        --input "$ZIP_PATH" \
        --output "$ONNX_PATH"
    echo "ONNX exported to: $ONNX_PATH"
fi

# ── Optional: Publish to Spaces ───────────────────────────────────────────
if [[ "$PUBLISH" == "--publish" ]]; then
    if [[ -z "${SPACES_KEY:-}" || -z "${SPACES_SECRET:-}" ]]; then
        echo ""
        echo "WARNING: --publish requested but SPACES_KEY / SPACES_SECRET not set. Skipping."
    elif [[ ! -f "$ONNX_PATH" ]]; then
        echo ""
        echo "WARNING: --publish requested but ONNX not found at $ONNX_PATH. Skipping."
    else
        echo ""
        echo "[publish] Uploading to Spaces and confirming..."
        python tools/publish_model.py \
            --onnx "$ONNX_PATH" \
            --name "$JOB_NAME" \
            --type "$MODEL_TYPE" \
            --api-base "${API_BASE_URL:-http://localhost:8000}"
    fi
fi

echo ""
echo "================================================================"
echo "Done."
echo "  SB3 checkpoint: models/pilot_ppo_${JOB_NAME}.zip"
[[ -f "$ONNX_PATH" ]] && echo "  ONNX model:     $ONNX_PATH"
echo "================================================================"
