# Runbook: specialist league with promotion gates and remote gate helpers

How the first specialist league (2026-11-08 to 2026-11-10) was built and run,
so the next league can reuse it. Design rationale:
`docs/superpowers/plans/2026-11-08-specialist-league.md`. Driver:
`code/tools/cloud_search/specialist_league.py`. Helper coordinator:
`code/tools/cloud_search/gate_helpers.py`. Offline check:
`code/tools/cloud_search/specialist_check.py`.

## 1. What the league does

One search (PIMC 320 sims x 4 worlds) model per archetype. Each **specialist**
warm-starts from one base generalist and only ever pilots its own archetype's
lists. The other seat never pilots that archetype, so there are no mirrors.

Generations are synchronous. In generation g, every specialist trains against
frozen opponents from generation g-1:

| Generation | Opponent mixture (600 self-play games per specialist) |
|---|---|
| 1 | 0.8 base model on both seats (only the learner seat's rows train) + 0.2 CP8 piloting the CP8 campaign archetypes |
| 2+ | 0.7 other archetypes' **champions**, each piloting its own lists + 0.15 frozen base + 0.15 CP8 |

Engine specialist mode is the opponent pool's `learner_decks` plus per-entry
`decks` (`code/digimon-engine/src/selfplay/opponents.rs`, `driver.rs`). The
learner seat always holds its archetype. Paired games (`game_idx / 2`) share
decks, and the first player is `game_seed % 2`. Opponent pools are frozen per
generation under `pools/gen<g>/<slug>.json`, and a resume refuses a pool that
differs.

### Promotion gate (why it exists)

Without a gate, "champion = latest generation" let worse models spread through
the league. The local paired check after gen 2 measured **0.431 for the gen 2
specialists against 0.521 for the gen 12 base, a difference of -0.090 (95% CI
-0.131 to -0.049)**. Only 5 of 18 archetypes improved.

After a specialist trains generation g:

- The new model (challenger) and the current champion each pilot the
  specialist's lists in the **same 120 seeded games**: 60 deck pairs, each list
  opening once and going second once.
- The opponent field is the frozen base model piloting every other archetype
  (`field: base`). The base model plays the gate as self-play when it is itself
  one of the two models.
- Both play with search 320/4, root noise 0 (`--root-noise-epsilon 0`) and
  temperature 0.
- Seed: `gate.seed + 1_000_003*g + 1009*index`, so each generation gets fresh
  games.
- Rule: promote **iff the paired mean score difference is > 0**. The user
  decided 2026-11-09 to keep this rule over requiring a margin. With 120 games
  the SE is about 0.05, so near-ties flip either way, while clear regressions
  are blocked.
- The record goes to `gates/gen<g>/<slug>.json`: challenger and champion
  hashes, both scores, the difference and its SE, truncations, and
  `champion_after`.
- The next generation warm-starts from the champion. A rejected model's
  **weights are dropped**, but its self-play games stay in the 3-generation
  replay window (`replay_window: 3`, 600 games per generation).

The gate can be turned on for a league that is already running. `enable-gate`
gates the already-trained generations in order and then continues. Opponent
pools for later generations use champions (`gated_pool`). A champion that is
still the base model merges into the base entry, because pool validation
rejects duplicate hashes. If the learner itself is still the base, that share
becomes self-play.

### Remote gates (helper hosts)

A gate costs more than a training generation: 240 full-budget games against
600 games played on a reduced self-play budget. On the 8-core box, gating was
**about 70% of the remaining wall-clock**. `enable-remote-gates` lets other
machines run gates:

- **Claims:** `gates/claims/gen<g>/<slug>` is created with `O_EXCL` and holds
  its owner (`controller` or `helper:<name>`). It is re-entrant for the same
  owner.
- **Controller (`run`):** with `gate.remote`, it does not gate inline. After
  training every specialist of generation g, it calls `settle_gates(g)`. That
  runs every unclaimed gate on the box (from the start of the archetype list),
  waits for the claimed ones (`league_state.stage = gate-wait`), and applies
  the warm start for each.
- **Coordinator (`gate_helpers.py`):** runs on the league host, outside
  Docker, stdlib only. A gate is eligible once its specialist has trained that
  generation (`state.json next_gen > g`) and, after generation 1, its gate on
  g-1 is decided. Helpers take the lowest generation first, from the **end** of
  the archetype list. For each gate the coordinator:
  1. rsyncs exactly the files the gate reads: the plan, `inputs/decks.json`,
     `inputs/opponents/*`, both models, and the g-1 gate record;
  2. runs `specialist_league gate-one` in the same image on the helper;
  3. copies back the gate folder, then the record (record last, so the
     controller never sees a record without its games).

  A failure or a 3 h timeout releases the claim, and the controller then runs
  that gate itself. When the coordinator starts, it releases stale `helper:`
  claims.
- Results are identical wherever a gate runs: same image id, same seeds, and
  both sides of a gate play on the same machine.

## 2. Layout on the league host

```
/opt/digimon-search/specialist-league-<date>/      (bind-mounted as /experiment)
  league-plan.json            hash-bound plan: image, archetypes, mixture, gate, recoveries[]
  league-plan.pre-*.json      plan snapshots taken before each recovery/enable step
  league_state.json           stage (generation | gate | gate-wait | complete), generation, specialist
  training.log                controller log (systemd append)
  gate-helpers.log            coordinator log
  inputs/                     base.{zip,onnx,onnx.meta.json}, decks.json, cp8_archetypes.json,
                              opponents/{base,cp8}.*   (hashes in plan.files; run refuses drift)
  pools/gen<g>/<slug>.json    frozen opponent pool per specialist and generation
  specialists/<slug>/generation/
    state.json                next_gen + prev (warm-start model)
    gen<N>/                   model.{zip,onnx,onnx.meta.json}, shards/, summary.json, ...
  gates/gen<g>/<slug>.json    gate record;  gates/gen<g>/<slug>/{challenger,champion}/shards
  gates/claims/gen<g>/<slug>  gate claim (owner)
```

Helpers mirror the parts they need under `/opt/league` (also mounted as
`/experiment`).

## 3. Procedure

### 3.1 Roster and decks (local)

1. Choose archetypes and 3-4 lists each. Every card must be admissible
   (`data/card_admissibility.json`, CLAUDE.md rule 34). Sources: the campaign
   deck pool, then the DigiLab API (`X-API-Key: $DIGILAB_API_KEY` header only).
   The 2026-11-08 roster is in
   `qa/archetype-qa/specialist-league-roster-2026-11-08.md` and
   `qa/archetype-qa/specialist-league/training-lists/`.
2. `python -m tools.cloud_search.specialist_league build-decks --campaign-decks
   <campaign decks.json> --roster <roster.json> --admissibility
   data/card_admissibility.json --extra <archetype> ... --out <inputs dir>`.
   It writes `decks.json` and `cp8_archetypes.json`.
3. Put the base model (`base.zip`, `.onnx`, `.onnx.meta.json`) in `inputs/`
   and `inputs/opponents/`, and add `opponents/cp8.*`.

### 3.2 Host and image

- **League host:** Hetzner ccx33 (8 dedicated cores, $0.266/h).
  `HCLOUD_TOKEN`; SSH with `~/.ssh/digimon_deploy`. The project is capped at 8
  dedicated cores until Hetzner support raises it (`docs/CLOUD_TRAINING.md`
  §B.2).
- **Image:** copy the source tree (origin/main plus any uncommitted overlay) to
  `/opt/digimon-search/league-source-<date>` and build with
  `code/tools/cloud_search/Dockerfile.cpu`. A full build recompiles the
  engine, about an hour under `nice`.
- **Python-only changes:** use a **thin image** instead: a 2-line Dockerfile
  `FROM <image id>` plus `COPY <files> /app/code/tools/cloud_search/`. It
  builds in seconds with a new id, so provenance is preserved.
- Every image change goes through a plan recovery record (`enable-gate`,
  `enable-remote-gates`). `run` refuses an image that differs from
  `plan.image`.

### 3.3 Prepare and run

```bash
L=/opt/digimon-search/specialist-league-<date>; IMG=sha256:...
docker run --rm --network=none -e DIGIMON_DEPLOYED_IMAGE=$IMG \
  --mount type=bind,src=$L,dst=/experiment $IMG \
  python -m tools.cloud_search.specialist_league prepare \
  --source-commit <commit+overlay> --base-label "<base>" --cp8-sha256 <hash> \
  --generations 4 --games 600
# optional, before or during the run: promotion gate (+ extend generations)
docker run ... $IMG python -m tools.cloud_search.specialist_league enable-gate \
  --source-commit <...> --generations 8 --reason "<why>"   # --games 120 --seed 61109000 defaults
```

`prepare` runs an 8-game health check. Run the controller as a systemd service
(`Restart=no`; a resume is a deliberate `systemctl start`):

```
ExecStart=/usr/bin/docker run --rm --init --name <name> --memory=24g --cpus=8 --stop-timeout=120
  --network=none -e DIGIMON_DEPLOYED_IMAGE=$IMG -e DIGIMON_ONNX_OPPONENT=0
  -e DIGIMON_SELFPLAY_TRACE_DECISIONS=1 -e DIGIMON_SEARCH_EVAL_CACHE_ENTRIES=0
  -e OMP_NUM_THREADS=1 -e MKL_NUM_THREADS=1 -e OPENBLAS_NUM_THREADS=1
  --mount type=bind,src=$L,dst=/experiment $IMG python -m tools.cloud_search.specialist_league run
StandardOutput=append:$L/training.log
```

`run` resumes at specialist granularity. A partly finished self-play stage
resumes with `--resume-selfplay`, and an interrupted gate resumes from its
saved batches. To change the image: stop the service, apply the recovery
command with the new image, `sed` the image id in the unit (it appears twice),
`daemon-reload`, then start.

### 3.4 Remote gate helpers (DigitalOcean)

1. On the league host, create `/root/.ssh/gate_helpers` (ed25519) and register
   its public key with DO (`DO_TOKEN`, `POST /v2/account/keys`).
2. Create the droplets with `POST /v2/droplets`:
   - region `nyc3` (near Hetzner Ashburn), size `s-8vcpu-16gb`,
     image `ubuntu-24-04-x64`, tag `digimon-gate-helper`;
   - SSH keys: `digimon-deploy` plus the host key;
   - cloud-init `packages: [docker.io, rsync]`.

   This account offers **no dedicated-CPU sizes**: 8 shared vCPU is the
   ceiling, with a droplet limit of 15.
3. From the league host, wait for `cloud-init status --wait`, then ship the
   image: `docker save $IMG | gzip -1` (~540 MB) piped to `ssh helper "gunzip |
   docker load"`. Check that the loaded id equals `$IMG`.
4. Write `helpers.json` (`[{"name": ..., "host": ...}]`). Copy
   `specialist_league.py` and `gate_helpers.py` to
   `<dir>/py/tools/cloud_search/`.
5. Stop the league, then run `enable-remote-gates --source-commit <...>
   --reason "<why>"` with the image that contains `gate-one`. Update the unit's
   image and start the league.
6. Start the coordinator **after** the controller has claimed its in-progress
   gate:

```
ExecStart=/usr/bin/python3 -m tools.cloud_search.gate_helpers --root $L \
  --helpers <dir>/helpers.json --image $IMG --ssh-key /root/.ssh/gate_helpers
Environment=PYTHONPATH=<dir>/py
StandardOutput=append:$L/gate-helpers.log
```

The coordinator exits when `league_state.stage` is `complete`. **Delete the
droplets then:** they bill until deleted (`DELETE /v2/droplets?tag_name=digimon-gate-helper`).

### 3.5 Monitoring

```bash
ssh -i ~/.ssh/digimon_deploy root@<host> 'L=...; cat $L/league_state.json;
  grep -h "^gate gen\|done in" $L/training.log | tail; grep -v "^---" $L/gate-helpers.log | tail;
  ls $L/gates/claims/gen*/'
```

Gate decision lines look like `gate gen 1 Venmon: challenger 0.050 vs champion
gen0 0.150 -> kept champion`. The 120 games of a gate are 30 shard manifests
per role. `dirichlet_epsilon: 0.0` in a gate manifest confirms the games ran
without noise.

### 3.6 Offline check (local, independent seeds)

`specialist_check.py` plays a fixed schedule: 40 games per archetype, the
candidate piloting A against the frozen base on other archetypes, both at
search 320/4 with no noise. It runs once for the baseline and once per
generation, and `compare` gives the paired difference with a CI per archetype:

```bash
python -m tools.cloud_search.specialist_check prepare --root R --engine <release cli> --inputs <inputs> --label genN --models <dir of genN onnx>
python -m tools.cloud_search.specialist_check run --root R --workers 6
python -m tools.cloud_search.specialist_check compare --baseline R0 --candidate R1
```

Locally this runs at about 48 s per game per worker, so 720 games take about
3 h. Its seeds differ from the gate's, so it is an independent measure of the
gated champions.

### 3.7 Matchup matrix (local)

`specialist_matrix.py` plays every unordered pair of archetypes: 4 seed-paired
game pairs per cell (each side opens once), lists cycling, search 320/4 on both
seats. Run it once with the base model on both sides (deck strength alone, reusable for every generation) and once per
generation with each archetype's champion:

```bash
python -m tools.cloud_search.specialist_matrix prepare --root R0 --engine <release cli> --inputs <inputs> --label base
python -m tools.cloud_search.specialist_matrix prepare --root R5 --engine <release cli> --inputs <inputs> --label gen5 --models <dir of champion onnx>
python -m tools.cloud_search.specialist_matrix run --root R5 --workers 12
python -m tools.cloud_search.specialist_matrix report --root R5 --baseline R0
```

18 archetypes give 153 cells and 1,224 games, about 2.3 h at 12 workers.
Field-wide lift is zero-sum: read a pilot's improvement against opponents that
are still on the base model. Findings and next-run changes are in
`docs/runbooks/specialist-league-next-run.md`.

## 4. Measured costs and timings (2026-11-09)

| Step | Time |
|---|---|
| Training generation per specialist, box (600 games + update) | ~11 min (652 s avg; self-play ~560 s, train ~75 s) |
| Gate per specialist, box (2 x 120 games) | ~14.5 min |
| Gate per specialist, DO s-8vcpu-16gb helper | ~37.5 min (about 0.4 of the box) |
| One generation, 18 specialists, box only, gated | ~7.6 h |
| One generation with 3 helpers | ~3.9 h (box training 3.3 h + the last specialist's gate) |

The 18-archetype, 8-generation league (gens 1-4 trained ungated, then gated
retroactively) cost about **$21**: ~41 h of ccx33 ($10.80) plus ~23 h of 3
droplets ($9.90). Without helpers it would have been about 62 h for $16.50.

Scaling notes:

- Once helpers are gating, **training on the box is the bottleneck**. A 4th
  droplet saves only about an hour.
- Cheap wins not yet built:
  1. Let the box take the last gates of a generation itself. It is idle in
     `gate-wait` and 2.5x faster than a droplet.
  2. Let `settle_gates` start the next generation's work while it waits.
- Bigger wins:
  - A larger Hetzner box after a core-limit increase (ccx53 32c / ccx63 48c
    finish a league in about 10-13 h for the same total cost).
  - Moving training of some specialists onto helpers (needs shared run state).

## 5. Gotchas

- **Long waits:** background `Bash` waits cap at 10 min unless given a timeout;
  run multi-hour local jobs detached (`nohup`).
- **`run` checks hashes:** it re-hashes every `inputs/` file against
  `plan.files`, and the image id against `plan.image`.
- **Opponent pools:** `validate_pool` (Python) and `load` (Rust) reject an
  opponent whose hash equals the learner's or another entry's. Gate and
  champion pools handle this with base merging or self-play. Any new pool shape
  must handle it too.
- **Paired games:** the gate's pairing check (`decide`) refuses games whose ids,
  decks or learner seat differ between challenger and champion. It is a real
  invariant check, not a formality.
- **Stopping the service:** `systemctl stop` makes the container exit non-zero,
  so `is-active` then reports `failed`. That is expected; resume with
  `systemctl start`.
- **Late user env vars:** a token added after the Claude app launched (e.g.
  `DO_TOKEN`) is missing from the session env. Read it with PowerShell
  `[Environment]::GetEnvironmentVariable('DO_TOKEN','User')`.
