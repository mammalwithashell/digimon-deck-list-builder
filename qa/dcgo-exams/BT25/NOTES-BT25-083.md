# NOTES — LadyDevimon (BT25-083)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-083`):
**4 clauses** — `effect#0`, `effect#1`, `effect#2`, `inherited#0`.
DCGO script: `BT25/Purple/BT25_083.cs` exists — the card is **not** `unavailable`.
All four clauses have a scenario; none is `unreachable`.

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT25-083#effect#0` | `[Digivolve] Lv.4 w/[Three Musketeers] in text or w/[TS] trait: Cost 3` | `BT25-083-effect0.yaml` (book `tm_bt25_083_pool.json`, deck `tm-aegiomon`) | lowers, asserts pass |
| `BT25-083#effect#1` | `[On Play] [When Digivolving] By placing 1 [Three Musketeers] trait card … <Draw 1>.` | `BT25-083-effect1.yaml` ([On Play] arm) | lowers, asserts pass |
| `BT25-083#effect#2` | `[When Digivolving] [When Attacking] [Once Per Turn] By trashing 1 Option card … you may use 1 [Three Musketeers] trait Option card from your trash with the cost reduced by 3.` | `BT25-083-effect2.yaml` ([When Attacking] arm) | lowers, asserts pass |
| `BT25-083#inherited#0` | `[On Deletion] You may play 1 level 4 or lower Digimon card with [Three Musketeers] in its text from your trash without paying the cost.` | `BT25-083-inherited0.yaml` | lowers, asserts pass (repaired 2026-09-16, below) |

## `effect#0` — why the base is a YELLOW Lv.4

LadyDevimon's printed purple circle is *also* "Lv.4 / cost 3", so on any
purple Lv.4 (Scopemon, BlackGatomon) both routes open at the same cost, no
prompt separates them, and the line would confirm nothing about the clause.
The base is **Aegiomon P-194** (yellow Lv.4, traits Shaman/TS, only `<Blocker>`
/ `<Barrier>`): the purple circle fails on colour and the clause is the only
legal route. DCGO's `AddSelfDigivolutionRequirementStaticEffect(..., level: 4)`
carries the default `cardColor: None`, and `AddDigivolutionRequirement.cs`
`GetEvoCost` applies no colour check in that case; our `alt_paths` `from:`
carries no colour either. Aegiomon is not in the shared pool, hence the
per-card book (shared 50 with BT24-081 and one EX7-070 swapped for P-194 ×2).

## `inherited#0` — repaired: the missing BeelStarmon cost row

The committed file (`c9c3ccf49`, 2026-09-13) no longer lowered:
`step 14: our engine asks a selection here … (pending kind: EffectChoice,
prompt: 'Choose which digivolution cost to pay')`. BeelStarmon BT25-085's
alt route ("Lv.5 w/[Three Musketeers] in text or w/[TS] trait: Cost 3")
landed in the card spec after the scenario (`9aa21ed0f`, 2026-09-15), so the
digivolve from LadyDevimon (purple Lv.5 / 4 printed; satisfies the alt) now
opens two distinct costs. DCGO asks too: `BT25_085.cs` registers the alt with
`ignoreDigivolutionRequirement: true`, and `Player.CanIgnoreDigivolutionRequirement`
returns `true` unless a "can't ignore" effect is in play, so its
`SelectCountEffect` offers `{3, 4}`. A **shared** `select: { value: 4 }` row
was inserted after the digivolve (4 keeps the line's 3 → -1 arithmetic) and
the assert index moved from 23 to 24. Every other line in this campaign that
digivolves into BT25-085 from a [TS]/[Three Musketeers] Lv.5
(`BT25-082-*`, `EX7-073-effect0`) should be re-lowered for the same reason —
not this stage's cards, not edited here.

## The memory-reset rule (measured while authoring `effect#0`)

The first draft played Aegiomon from the default 3 (→ -1) and had P1 overspend
next turn; it failed to lower because P1's turn started at **1**. That is
rules-correct, not an engine finding: `general_rule.pdf` §6-2-1-1 quotes
"[Start of Your Turn] If you have 2 or less memory, set it to 3" as a *card*
effect (there is no automatic reset), DCGO sets 3 only on a pass
(`AutoProcessing.cs::EndTurnProcess`, `Passed && Main`), and our engine agrees.
The committed line crosses every turn boundary by a pass or an overshoot of
3+ (P1 overspends on T2 and T4; P0 passes at 1 on T3), so no engine ever has
to reset anything.

## Adversarial pre-Unity review — prompt sequences re-derived from `BT25_083.cs`

- `effect#0`: after the digivolve, the OP/WD shared ActivateClass is gated by
  `AdditionalActivateCondition` (a `[Three Musketeers]` **trait** card in hand
  or trash — P0 holds only `[TS]` Tamers, trash empty) and the WD/WA shared
  one by `AdditionalActivateCondition2` (an Option under an own Digimon —
  none). Neither activates; no `MultipleSkills`, no `OptionalSkill`, no
  `SetIntSelection`. **Our side is not silent** — see the finding below; the
  two vacuous sim-side prompts are answered `sim_only` and put nothing on the
  wire, so the DCGO trace stays prompt-free from the digivolve to the pass.

## Engine-side finding (card YAML): vacuous `[When Digivolving]` triggers

Measured at lowering on `effect#0`. `code/digimon-engine/cards/bt25/BT25-083.yaml`
carries **neither** of DCGO's activation gates, so on any digivolve into
LadyDevimon our engine triggers both `[When Digivolving]` clauses
unconditionally and then:

1. parks a `TriggerOrder` between them (`'Choose which triggered effect to
   resolve next (2 pending)'`) even though neither has material — the tuck's
   union pick has zero candidates and the WD/WA clause has no Option source;
2. parks the WD/WA clause's `optional: true` gate (`Replacement`, `'You may
   activate BT25-083's triggered effect'`) with nothing behind it.

DCGO gates clause 1 on `AdditionalActivateCondition` (a `[Three Musketeers]`
trait card in hand or trash) and clause 2 on `AdditionalActivateCondition2`
(an own Digimon whose digivolution cards hold an Option) and activates
neither, asking nothing. Both of our prompts are outcome-void — the order of
two no-op effects and a "may" over an unpayable cost change no reachable
state — so under rule 17's *choice* test they are over-exposures of the same
family as the `OrderedPermutation`-at-N=1 row in `docs/DCGO_EXAM.md` and the
"spurious optional gate" logged in `EX7/NOTES-EX7-070.md`. The fix is a
`condition:` on each clause mirroring the DCGO gates (clause 1:
`count_gte { of: you, zone: [hand, trash], trait_has: "Three Musketeers" }
n: 1` — the idiom `BT25-082.yaml` already uses for its inherited WA clause;
clause 2: `any_permanent { of: you, zone: [battle_area], kind: digimon,
source_count { filter: { kind: option }, at_least: 1 } }` — the idiom
`BT21-074.yaml` uses). **Not applied here** (exam stage: triage and report);
it belongs to the card-fix gate (rule citation + failing-then-passing test +
`cards_behavioral` green). The scenario folds both rows `sim_only`, which is
sound only because the fold hides no ordering or outcome — once the YAML is
gated, the two rows must be removed or the line stops lowering.
- `effect#1` / `effect#2` / `inherited#0`: sequences as documented in each
  file's header (authored 2026-09-13, re-lowered 2026-09-16 unchanged apart
  from the cost row above).

## Update 2026-09-18 (oracle) — 4 clauses: 2 confirmed, 2 diverged (aborts, upstream)

`effect#0` / `effect#1` diffed CLEAN. `effect#2` and `inherited#0` aborted
before reaching their clause, both on **BeelStarmon BT25-085 being a dual card
in DCGO and a plain Digimon in ours** (`NOTES-BT25-085.md`): DCGO resolved
`play: BT25-085` as an Option use (board stayed empty, so P-180 had no tuck
target), and took `digivolve ... using: BT25-085` at cost 0 with the Option
face's `[Main]` firing. Neither is a LadyDevimon finding; both lines need a
non-dual Option host / Lv.6 (or the BT25-085 card-data fix) and a re-run.

## Update 2026-09-18 (triage) — `effect#2` re-authored and re-measured: CONFIRMED

Class of the original abort: **our bug, upstream card data** — BeelStarmon
(BT25-085) is a DUAL card (official Bandai DB "DUAL Effect"; `BT25_085.cs`
Option face) and `data/cards.json` typed it `card_kind: 0`; fixed in
`20df249e6`. Nothing in LadyDevimon (BT25-083)'s YAML or the engine changed.
After that fix `play: BT25-085` no longer lowers (correct), so the line was
re-hosted on the non-dual **BeelStarmon (X Antibody) EX7-073** (12 cost, no
[On Play]).

Two oracle runs:

- `exam-BT25-083-effect2-r2` (recording `20260918T091206Z_3d49d468…`) aborted at
  step 19, `expected 'SelectPermanentEffect' but DCGO asked 'OptionalSkill'`.
  **Attacker slot-addressing artefact, not a clause finding** (same family as
  EX7-070#effect#2 / EX7-071#effect#2): `attack: field.N` reaches DCGO as a
  COMPACT index into `Player.GetFieldPermanents()` (frame-id order,
  `InputDriver.cs` ~343-363), and `CardSource.PreferredFrame()` seats P0's
  Digimon centre-out (frame 4, 3, 5, …). With two Digimon DCGO's order is the
  reverse of our play-order battle area, so BeelStarmon (X) attacked and its
  optional [When Attacking] asked.
- `exam-BT25-083-effect2-r3` (recording `20260918T091629Z_4f46c937…`): a second
  EX7-073 is played as slot ballast so LadyDevimon is the THIRD Digimon —
  index 2 on both sides. Diff **CLEAN** (26 rows). DCGO's trace:
  SelectPermanentEffect → SelectCardEffect (trash P-180) → SelectCardEffect
  (use P-180 from trash, memory 3 → 0 = 6 − 3) → P-180's tuck
  SelectPermanentEffect → `effect_activation` P-180 then BT25-083.

**Reusable authoring rule:** on a line with exactly two own Digimon, `attack:` /
`digivolve:` `field.N` addresses the OTHER Digimon in DCGO. Attack with the
only Digimon, or with the third-played one.

BT25-083 now: 4 clauses — 3 confirmed, 1 diverged (`inherited#0`, still the
BT25-085 dual-card abort; needs the same re-authoring), 0 unmeasured.

## `inherited#0` triage (2026-09-18) — the r1 abort was a SLOT-ADDRESSING artefact, not the dual-card gap

r1 (recording `20260918T072334Z_00d54ffc…`) aborted "step 15 expected prompt
'SelectCountEffect' but DCGO asked 'SelectPermanentEffect'" and was first read as
the BT25-085 dual-card data gap. The recording says otherwise: `board_p0` at the
digivolve row is `["BT25-083", "BT21-071"]`. DCGO seats Digimon centre-out
(`CardSource.PreferredFrame`: frames 4, 3, 5, …) and the harness addresses the
battle area by COMPACT frame order (`InputDriver.FieldSlotToFrameId`), so on a
two-Digimon board the second Digimon played is slot 0 on DCGO and slot 1 on
ours. `digivolve: { from: field.1 }` named **Scopemon** on DCGO; a Lv.6 cannot
digivolve onto a Lv.4, so DCGO resolved the dual card's `PlayCardAction` as an
Option use (Scopemon satisfies the Use Req.) and ran Fly Bullet's `[Main]`. The
data fix (20df249e6) alone would NOT have cleared this line.

Class: scenario artefact (neither engine wrong; no card/engine change). Line
re-authored so Scopemon attacks into Birdramon and dies on T5 BEFORE LadyDevimon
is played — LadyDevimon/BeelStarmon is then the only P0 Digimon (`field.0` on
both engines). Oracle job `exam-BT25-083-inherited0-r2`, recording
`20260918T092510Z_577a717f…`: diff CLEAN (24 of 25 rows), DCGO
`effect_activation` BT25-083 "Play 1 level 4 or lower [Three Musketeers]
Digimon" `executed: true`, Scopemon revived at no cost. Verdict **confirmed**
(BT25-083 is now 4/4). Rule of thumb for this campaign: never address a slot on
a TWO-Digimon board — one Digimon, or make the addressed one the third.
