# NOTES -- Landramon (EX10-028)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX10-028`):
**2 clauses** -- `effect#0` ([On Play] [When Digivolving] "by trashing" grant), `inherited#0`.
DCGO script exists (`EX10/Black/EX10_028.cs`); card is not `unavailable`.

EX10-028: 2 clauses: 1 confirmed, 1 diverged, 0 unreachable, 0 unavailable, 0 unmeasured

Book: `rocks_pool_ex10_028.json` = `rocks_pool.json`'s `rocks-exam` deck unchanged plus
`rocks-exam-tumble` (same 50, eggs 4x EX10-003 Tumblemon instead of ST5-01).

| Clause | Scenario | Oracle round-trips | Verdict |
|---|---|---|---|
| `EX10-028#effect#0` | `EX10-028-effect0.yaml` | 1 | confirmed (CLEAN, 13 of 16 ours / 15 dcgo rows compared) |
| `EX10-028#inherited#0` | `EX10-028-inherited0.yaml` | 1 | diverged (known finding, below) |

## effect#0 -- confirmed, [When Digivolving] timing only
Line: Tumblemon (EX10-003, Rock egg) -> Sunarizamon EX10-025 (cost 0) -> Landramon (cost 2).
Optional cost trashes Tumblemon (Rock; it has NO "when effects trash this card" trigger, unlike
every Sunarizamon, so the known source-trash-trigger ordering finding does not enter). Grant
target is Landramon itself. DCGO sidecar: step 15 Landramon sources [EX10-025], trash
[EX10-003]; step 17 Landramon DP 7000 (4000 + 3000). Ours agreed on the whole line.
Prompt shape folded: DCGO asks carrier SelectPermanentEffect then SelectCardEffect; ours one
SourceMulti pick (authored as dcgo_only / sim_only rows).
NOT exercised / not measured by this verdict:
- the [On Play] timing (same C# body, separate `ActivateClass`; not run);
- <Reboot> and <Blocker>: the DCGO state sidecar rows carry no keyword field, so only the
  +3000 DP half of the grant is compared;
- declining the optional gate, and the no-qualifying-source case.
No `assert:` block was backfilled.

## inherited#0 -- DIVERGED: resolution order (triage: our bug, rules-supported; same finding as EX10-025#inherited#0)
Line: EX11-038 x2 fill the trash (Landramon A from hand, Gotsumon BT14-009 from hand); EX10-025
places both under itself; EX11-038 #A digivolves into Landramon B (the T1 draw); B's
[When Digivolving] cost trashes Landramon A from EX10-025's sources, firing A's inherited
delete on p1's BT16-082 Ukkomon (play cost 3). Round-trip 1 ran to completion.
- DIVERGED at step 26: `p0.field[1].dp: ours=4000 dcgo=7000`. DCGO rows: step 26 Landramon B
  4000 with A in trash and Ukkomon alive; step 28 B at 7000 (own grant resolved); step 30
  Ukkomon in p1.trash. So DCGO finishes Landramon B's own grant BEFORE the inherited trigger
  resolves; our engine resolves the trigger mid-effect (asks the delete pick first, B's grant
  pick last). Only that one row diverged (`--all-diffs` printed only the lead); the end
  outcome (Ukkomon deleted, A in trash, B +3000) is the same -- ordering, not outcome.
- Rule: general_rule.pdf p.25 section 15-8-3-2 "Trigger-type effects can't activate during
  the processing for a rule or effect" (read directly this run). DCGO's order is the
  rule-supported one. Known finding: G-ENGINE-SOURCE-TRASH-TRIGGER-RESOLVES-MID-EFFECT
  (`qa/dcgo-exams/P/NOTES-P-180.md`). Not fixed here.
- The trasher is Landramon B's own clause, so this line CANNOT avoid the known finding. The
  first-choice line used EX11-038 (only "Draw 1" after its cost) as trasher to avoid it, but
  was NOT authorable: EX11-038's cost prompt is a UnionZone, and a digivolution-SOURCE pick
  inside a UnionZone with 2+ qualifying sources resolves neither by `cards:` (searches
  hand/trash only, `selection_resolve.rs` ~L680) nor by `value:` (a count payload, requires
  `candidates`). A line trashing a source via UnionZone is therefore unmeasurable with
  today's tooling when 2+ sources qualify.
