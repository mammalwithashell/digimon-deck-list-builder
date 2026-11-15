# Live DCGO on Linux cloud: 2026-11-14

Source base: `8b25ab68ed4872f862449c70c7b862eb57d482d8`. Official DCGO 1.19.3
IL2CPP client, Wine 10, DXVK 2.3.1, Mesa Lavapipe 25.0.7, Xorg dummy, MelonLoader
0.7.3, Windows .NET 6.0.36. See [the runbook](../../../../docs/runbooks/dcgo-linux-cloud.md).

`cloud-bot-raw.jsonl` is an actual game recorded in this cloud session, not an
existing repository fixture or simulator-generated trace. The unmodified mod
automatically selected the exported ST1 deck, chose first player, kept the local
hand, passed breeding/main, and surrendered when the bot entered its breeding
choice. It converted without truncation: 5 decisions, 0 inferred. Replay reports
1 pass, 0 partial, 0 fail, 0 skipped. This is a short plumbing smoke test;
no ST1-15 effect was exercised and no clause was promoted.

The original mod SHA256 was
`fd5a0691372ab6286b73d8334770f9f7804cac895b2130231bf180462c285e68`.
`SHA256SUMS` records the downloads and the **updated** mod after the fix that
waits for the bot's transient selection instead of surrendering immediately.

`cloud-bot-fixed-raw.jsonl` is the subsequent live run through turn 6. It captures
Dracomon's paid digivolution, Gaia Force, and Phoenixmon. Before the converter
fix, all three disappeared because they lacked queued packets; the resulting
recording failed with `actor_mismatch` (retained as
`before-converter-fix-parity.json`). The corrected converter produces 16 decisions
with 4 inferred choices. Replay passes with 2 redundant inferred passes skipped.
The bot's move/attack on turn 4 were not hooked even though visible on screen;
this is **not full state parity or complete training data**. The small regression
test verifies the missing plays are retained and human/PvP/free-effect filtering
is preserved. Three tests pass.

`ST1-15-sim.log` records the new two-target exam's 15-step lowering, emitted
source-harness job, and 6 passing assertions. Those expectations were authored
from card text/source behavior and checked in our engine. The job was not
consumed by a live compiled Oracle player. `ST1-15-binding.json` records the
current full denominator and the invalidation of historical verdicts: 2 total,
0 confirmed, 0 diverged, 0 unreachable, 0 unavailable, 2 unmeasured; no in-scope
orphan scenarios. Both clause IDs are retained in that report.

Raw, converted and report files are deliberately distinct. Replay of an action
recording does not by itself prove per-prompt state equality with an Oracle
sidecar. The converter's inferences and any partial result must remain visible.
