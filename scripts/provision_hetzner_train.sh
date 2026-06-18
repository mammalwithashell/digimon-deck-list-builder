#!/usr/bin/env bash
# Provision a Hetzner Cloud dedicated-vCPU box and launch a Digimon floor run.
#
# DO is tier-capped at 4 vCPU on this account; Hetzner CCX gives 8/16/32
# dedicated vCPU on demand (no tier wall) for the env-bound MLP-vs-greedy
# floor runs. This mirrors docs/CLOUD_TRAINING.md "Path B" plus the launch.
#
# Usage:
#   export HCLOUD_TOKEN=...            # Hetzner Cloud API token (project-scoped)
#   scripts/provision_hetzner_train.sh <name> <vcpus> <run-args-file>
# e.g.
#   scripts/provision_hetzner_train.sh digimon-single 16 single
#
# It is intentionally step-by-step / idempotent-ish so it can be run by hand and
# inspected between steps rather than fire-and-forget.
set -euo pipefail

NAME="${1:?server name, e.g. digimon-single}"
WANT_VCPU="${2:-16}"
RUNKIND="${3:-single}"   # single | bo3
IMAGE="ghcr.io/mammalwithashell/digimon-trainer:training-v0.33"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/digimon_deploy}"
LOCATION="${LOCATION:-nbg1}"
: "${HCLOUD_TOKEN:?set HCLOUD_TOKEN}"
export HCLOUD_TOKEN

echo "== pick the cheapest dedicated (ccx) server-type with >= $WANT_VCPU vCPU =="
TYPE="$(hcloud server-type list -o noheader -o columns=name,cores,memory \
  | awk -v want="$WANT_VCPU" '$1 ~ /^ccx/ && $2+0 >= want {print $2, $1}' \
  | sort -n | head -1 | awk '{print $2}')"
echo "selected server-type: ${TYPE:?no ccx type with >= $WANT_VCPU vCPU found}"

echo "== ssh key =="
hcloud ssh-key describe "$NAME-key" >/dev/null 2>&1 \
  || hcloud ssh-key create --name "$NAME-key" --public-key-from-file "${SSH_KEY}.pub"

echo "== firewall (SSH only inbound) =="
hcloud firewall describe digimon-train >/dev/null 2>&1 || {
  hcloud firewall create --name digimon-train
  hcloud firewall add-rule digimon-train --direction in --protocol tcp \
    --port 22 --source-ips '0.0.0.0/0' --source-ips '::/0'
}

echo "== create server =="
hcloud server describe "$NAME" >/dev/null 2>&1 || hcloud server create \
  --name "$NAME" --type "$TYPE" --image ubuntu-24.04 --location "$LOCATION" \
  --ssh-key "$NAME-key" --firewall digimon-train
IP="$(hcloud server ip "$NAME")"
echo "server $NAME up at $IP ($TYPE)"

echo "== wait for ssh =="
for i in $(seq 1 30); do
  ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 root@"$IP" true 2>/dev/null && break
  sleep 5
done

echo "== install docker + login ghcr =="
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no root@"$IP" '
  command -v docker >/dev/null || (curl -fsSL https://get.docker.com | sh)
'
# GHCR pull: pass GHCR_USER / GHCR_PAT in env if the package is private.
if [ -n "${GHCR_PAT:-}" ]; then
  ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no root@"$IP" \
    "echo '$GHCR_PAT' | docker login ghcr.io -u '${GHCR_USER:-mammalwithashell}' --password-stdin"
fi

echo "== stage data =="
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no root@"$IP" 'mkdir -p ~/digimon-training'
rsync -az -e "ssh -i $SSH_KEY -o StrictHostKeyChecking=no" \
  data/ root@"$IP":~/digimon-training/data/

echo "== launch =="
case "$RUNKIND" in
  single) MF=single; RUN=starter_floor_single_v7 ;;
  bo3)    MF=bo3;    RUN=starter_floor_bo3_v4 ;;
  *) echo "unknown run kind $RUNKIND"; exit 1 ;;
esac
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no root@"$IP" "
  docker pull $IMAGE
  docker rm -f digimon-train 2>/dev/null || true
  docker run -d --name digimon-train --restart=no \
    -v ~/digimon-training/data:/app/data \
    -e PYTHONUNBUFFERED=1 \
    $IMAGE \
    python -m digimon_gym.agents.pilot_training --generalist --timesteps 1000000 \
      --lr 0.0003 --match-format $MF --eval-freq 50000 --eval-episodes 20 \
      --curriculum-seed 123 --eval-seed 999 --save-dir /app/models \
      --log-dir /app/runs/$RUN --record-games eval --record-games-max 300 \
      --set run_name=$RUN --set tensor_profile=standard_lite_deck_v2 \
      --set reward_profile_override=starter1_6_flat --opponent greedy \
      --archetypes 'ST-1 Gaia Red' 'ST-2 Cocytus Blue' \"ST-3 Heaven's Yellow\" \
      'ST-4 Giga Green' 'ST-5 Machine Black' 'ST-6 Venomous Violet' \
      --set n_envs=$WANT_VCPU --set vec_env_backend=subproc
"
echo "launched $RUN ($MF, n_envs=$WANT_VCPU) on $NAME / $IP"
