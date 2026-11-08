# Frozen checkpoint evaluation review

Updated November 7, 2026 from independently reviewed evidence completed on
November 6. Runtime source is 82c2877f88ac071e97506c6eb7419c25624d3566.
This documentation update follows that qualified source. The evidence index
and exact hashes are in [validation.json](validation.json).

The two dispatched Windows held-out panels completed and were independently
reviewed and archived. The owner recorded 480 paired requests / 960 games,
zero rejected requests and 480 matched zero native exits. Saved terminal
evidence confirms the Windows host lock was released. The monitoring automation
was removed after completion. OpenSpec remains ACTIVE at 21/35 tasks.

## Verified held-out results

Both actors used closed-list search with 320 total simulations, four worlds,
batch size four and zero root noise. Each panel contains 120 four-game blocks,
with balanced seats and opening order. Results below are from CP20's perspective.

| Opponent | Wins | Losses | Games | Win rate | Four-game-block 95% interval |
| --- | ---: | ---: | ---: | ---: | ---: |
| CP12 | 231 | 249 | 480 | 48.125% | 43.958% to 52.292% |
| CP16 | 253 | 227 | 480 | 52.708% | 48.958% to 56.458% |

Both intervals include 50%, so the ordering among these checkpoints remains
uncertain. The panels compare checkpoints while both use search. A controlled
comparison against policy-only play is still needed to estimate search lift.

The 23 held-out lists span 12 archetypes and were excluded from the retained
weight-training provenance. They had prior evaluation exposure. Light Hybrid
has no measured held-out-list coverage in this source.

CP20 vs CP8 retains its independently complete 480-game panel from a separate
prior runtime. Its evidence is indexed separately. Fourteen earlier complete
panels / 7,280 games await corrected-rules eligibility. Five previously claimed
incomplete panels remain unresolved. These limitations prevent a complete
four-checkpoint ranking or a finished 22-panel study.

## Qualification and implementation

The latest production repair preserves a compiled predicate path for breeding
permanent selection in closed-list search. Opaque callbacks retain their
information-safety guards. The scoped runtime commit changes two production
files and one regression-test file.

The source-specific Windows/Linux qualification passed all 134 excluded
reference requests / 268 games per platform with exact non-timing parity.
The Linux evidence retains separate 71-request and 63-request cohorts.
Independent software, original prior and saved-root proofs are bound by the
qualification record. These reference games are excluded from strength scores.
All five required CI checks passed for the runtime source at launch.

The scored Windows runtime is SHA256
44eb7aa3cde0cb9300c5d57a6cd1fcbe7fa889250867270bd6f160b04cc68014.
The qualification source manifest is SHA256
274065fdc85509cd0b33886cb84abd031514abde9c86baeb33eb1cddfefd01c1.

## Remaining study work

The planned study contains 22 panels / 11,200 games, covering closed-list
rankings, matched known-list controls, within-checkpoint information-regime
comparisons and held-out comparisons. Mulligan diagnostics, search-opportunity
telemetry and opt-in Tamer-first baselines have separate protocols.

Next, reconcile historical corrected-rules eligibility and the incomplete
panels against the planned matrix using their cached proofs. Any further scored
attempt needs an explicit scope and fresh qualification/capacity assessment.
The completed Windows dispatch has exhausted its authorized scope.

The PR remains draft. The open task list includes full-study reporting, matched
mode comparisons, checkpoint ranking, final runtime analysis and the
next-experiment decision. Source-specific qualification and two completed panels
supply evidence for this portion of the study.

## Evidence and validation

The validation index retains the earlier packaging/test records and a current
evaluation section with exact proof hashes, result denominators and limitations.
Evidence paths are relative to the operational workspace's runs directory.
A fresh Git clone contains the summaries and indexes. The original workspace
retains models, binary/source bundles, claims, receipts, failures and archives.

The terminal proof is
runs/ex13-option-grouped-host-allocation-20261102/marcus-breeding-filter-two-panel-scored-terminal-review-v1-20261106/verified.json,
SHA256 576983d709b10bb38a30074f643b3e1c1a7229f642855c0560ca79d45afe1c6b.
Qualification is
runs/ex13-option-grouped-host-allocation-20261102/marcus-compiled-breeding-filter-cross-platform-qualification-v1-20261106/verified.json,
SHA256 e2739b63dafdd13d17fd5e62bb0037b06e65ceb42dfa1ffbbe2796dfda383759.

Earlier compile, fixture, native and strict-audit failures remain preserved in
their original evidence directories and the historical validation records.
The November 1 review snapshot is retained in Git history. The current scored
summaries above use the independently reviewed November 6 proof caches.
