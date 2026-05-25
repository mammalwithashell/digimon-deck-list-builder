#!/bin/bash
# runpod-start.sh — keep the container alive and serve SSH for RunPod's proxy.
#
# RunPod injects the user's public key as the PUBLIC_KEY env var. We write
# it into /root/.ssh/authorized_keys, start sshd, then sleep forever so
# RunPod's SSH proxy can connect.
#
# For Path B (Hetzner one-shot trainer), this script is overridden by the
# explicit `python tools/run_training_job.py <config>` command on the
# `docker run` line — sshd is never started in that scenario.
set -euo pipefail

# Install the runtime's bundled SSH key (injected by RunPod from the pod
# creation request). Idempotent: rerunning the container restarts cleanly.
if [[ -n "${PUBLIC_KEY:-}" ]]; then
  mkdir -p /root/.ssh
  chmod 700 /root/.ssh
  # RunPod sometimes sends multiple keys separated by blank lines; the
  # default key plus any user-added keys. Pass through unchanged.
  echo "${PUBLIC_KEY}" > /root/.ssh/authorized_keys
  chmod 600 /root/.ssh/authorized_keys
fi

# sshd needs the host keys generated on first boot.
if [[ ! -f /etc/ssh/ssh_host_ed25519_key ]]; then
  ssh-keygen -A
fi

# Run sshd in the foreground as PID 1's child but keep PID 1 a no-op
# sleep so a transient sshd crash doesn't terminate the pod.
mkdir -p /run/sshd
/usr/sbin/sshd
exec sleep infinity
