# ops/training — cloud training host infra

This directory holds the **observation** layer for cloud training hosts.
The trainer container itself is invoked as a one-shot `docker run` and is
deliberately not declared as a compose service here (we want loud failure,
not restart loops).

## What's here

- `docker-compose.watch.yml` — TensorBoard sidecar. Mounts `./runs/` read-only,
  serves the UI on `:6006`. Restart-on-failure.

## How it fits together

```
training droplet
├── ./runs/                 ← trainer writes, watcher reads
├── ./models/               ← trainer writes
├── ./data/                 ← cards.json + deck_library.json (read-only)
├── ./training_jobs/        ← job configs (read-only)
│
├── docker run --rm digimon-trainer …  ← one-shot, exits on completion
└── docker compose -f ops/training/docker-compose.watch.yml up -d
                                       ← long-lived, survives trainer restarts
```

## Reach

Port 6006 should only be reachable over Tailscale. The cloud-provider firewall
(Hetzner Cloud Firewall, DO Cloud Firewall) blocks inbound `:6006` from the
public internet; Tailscale's WireGuard tunnel is the only legitimate path.

See [docs/CLOUD_TRAINING.md](../../docs/CLOUD_TRAINING.md) for the end-to-end
provisioning runbook.

## Why no trainer service?

A trainer compose service would imply `restart:` semantics. A 13-hour training
job that silently restarts on a transient failure is worse than a job that
exits loudly and waits for you to look at it. Keep the trainer as a manual
`docker run`; keep the watcher (which we DO want to survive reboots) as
compose.
