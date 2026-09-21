# NOTES — Dinobeemon (BT16-077)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT16-077`):
**5 clauses** — `effect#0` (`[DNA Digivolve] Purple Lv.4 + red Lv.4: Cost 0 …`),
`effect#1` (`<Raid>`), `effect#2` (`<Partition (purple Lv.4 & red Lv.4)>`),
`effect#3` ([When Digivolving] "If DNA digivolving, you may play … [Free] …
from your trash … Then, 1 of your Digimon may gain <Rush> for the turn and
attack a player"), `inherited#0` (inherited `<Partition …>`).
DCGO script: `BT16/Purple/BT16_077.cs` exists — the card is **not** `unavailable`.

**Status: 5 clauses — 4 scenarios, 1 `unreachable` (`effect#0`).** Book:
`qa/dcgo-exams/BT16/tm_bt16_077_pool.json` (decks `tm-purple-line`,
`tm-quiet-red` for effect1/effect3; `tm-partition`, `tm-quiet-gaia` — added on
the resumed audit — for effect2/inherited0).

| Clause | Scenario | Sim-only | Expected at the oracle |
|---|---|---|---|
| `effect#0` | — | — | **unreachable** — no DNA verb on either wire |
| `effect#1` | `BT16-077-effect1.yaml` (REPAIRED) | lowers, asserts pass | clean |
| `effect#2` | `BT16-077-effect2.yaml` (NEW) | lowers, asserts pass | clean (our extra source pick is `sim_only`) |
| `effect#3` | `BT16-077-effect3.yaml` (REPAIRED; second sentence only) | lowers, asserts pass | **predicted divergence — card YAML** |
| `inherited#0` | `BT16-077-inherited0.yaml` (NEW) | lowers, asserts pass | clean (same) |

## `effect#0` — `unreachable`: the exam cannot declare a DNA digivolution

- `code/tools/dcgo-harness/src/exam/scenario.rs` fixes the `do:` verbs to
  `hatch, pass, move, play, digivolve, attack, main, link, select`
  (`STEP_VERBS`); there is no DNA verb and `exam::lower::matches_intent` has
  no `DnaDigivolve` arm, so our engine's DNA action bits cannot be named.
- DCGO's side refuses the range outright:
  `Assets/Scripts/Script/Harness/InputDriver.cs` ~277, "Anything else -- DNA
  digivolve, … -- returns null … and the job aborts".

DCGO does script the condition (`AddJogressConditionClass`, purple Lv.4 + red
Lv.4, cost 0), so the oracle exists; the wire cannot reach it. Tooling gap
`G-TOOLING-EXAM-NO-DNA-VERB` (`docs/RUST_ENGINE_GAPS.md`). This also makes the
FIRST sentence of `effect#3` ("If DNA digivolving, you may play 1 level 5 or
lower [Free] Digimon from your trash") unreachable — see below. Never a silent
skip, never `confirmed`.

## `effect#2` / `inherited#0` — `<Partition>` reached WITHOUT a DNA verb

Partition needs "each of the specified digivolution cards" — a purple Lv.4 AND
a red Lv.4 (`PartitionCondition(4, Purple)`, `(4, Red)`) — and an ordinary
digivolution line holds exactly one Lv.4, so the crashed draft (and this
audit's first pass) had both clauses down as DNA-only. They are not: Scopemon
BT21-071's [On Play] places "1 card with the [Appmon] or [Three Musketeers]
trait … as 1 of YOUR DIGIMON's bottom digivolution card" — any own Digimon —
and Effecmon BT22-009 is a RED Lv.4 with the [Appmon] trait. So:

1. Dinobeemon digivolves onto a hard-played vanilla Youkomon ST6-07 (purple
   Lv.4) through its ordinary purple circle;
2. Scopemon is played and tucks Effecmon under Dinobeemon (`effect2`), or
   under the vanilla Phoenixmon ST1-10 that was digivolved on top of it
   first (`inherited0`, so only the INHERITED copy can answer);
3. P1's Gaia Force ST1-16 (an opponent's effect: not "your effects", not a
   battle) deletes the stack; P1's red Tai Kamiya pays its colour.

Every slot-addressed step happens while that seat has ONE Digimon; the
two-Digimon picks (tuck host, Gaia Force target) are identity picks.

Prompt shape (`KeyWordEffects/Partition.cs` + `CardEffectFactory/…/Partition.cs`):
`WhenRemoveField`, `SetUpActivateClass(…, -1, TRUE, …)` → ONE `OptionalSkill`;
`PartitionClass.Partition` opens a `SelectCardEffect` per condition ONLY when
that condition has more than one matching digivolution card — here one each,
so none; both cards are played free, Effecmon's [On Play] delete finds no
target (P1 fields a Tamer only) and asks nothing.

### ENGINE FINDING — our Partition did not enforce its slots (RESOLVED 2026-09-20)

`G-ENGINE-PARTITION-SLOT-ENFORCEMENT-DEFERRED` (`docs/RUST_ENGINE_GAPS.md`) —
**RESOLVED 2026-09-20** in the three-musketeers-2 close-out. The keyword body now
reads the card's printed slot specs (`CardEffect::partition_slots`, answered by the
YAML's `kind: partition` → `sources:`), gates the trigger on a complete slot
assignment (general_rule.pdf §16-28-1; DCGO `CardEffectFactory/.../Partition.cs:145-159`),
masks every pick that would strand a slot, refuses a partial answer (§16-28-6) and
takes a forced last card without a prompt (DCGO `KeyWordEffects/Partition.cs:89,117`).
Both lines here re-diffed CLEAN against their preserved sidecars afterwards and stay
`confirmed`; their sim-only row is now a ONE-card pick over the two specified cards
only. The original measurement is kept below as the record of what was wrong.

`lower_partition.rs` documents `_sources` as deferred; measured here it is a
rules-visible gap, not a cosmetic one. After the `Replacement` accept our
engine parks a generic `CountCappedMultiSelect { min: 1, max: 2 }` "select 2
cards to play" over ALL the digivolution cards. Two negative probes (scratch
copies, not committed):

- `inherited0` with the pick answered `[BT16-077, ST6-07]`: our engine PLAYS
  DINOBEEMON (a Lv.5 that matches neither slot) and trashes Effecmon. "1 each
  of the specified cards" (general_rule 16-28: all-or-nothing across the
  specified set) admits no such play; DCGO's slot lists cannot
  contain it.
- `effect2` with the pick answered `[ST6-07]` alone: our engine plays ONE card
  and trashes the other. "1 each" is not "up to"; DCGO auto-takes both.

The committed lines pick the two legal cards, `sim_only` (DCGO asks nothing at
one candidate per slot), so they are expected CLEAN — the finding lives here,
not in a verdict. The fix is per-slot candidate lists (one pick per slot, a
pick parked only when a slot has >1 candidate — which also removes the
sim-only row).

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

`effect1` / `effect3` were on disk, uncommitted, with `assert: []`.

### Both files would have aborted at step 4: wrong `expect:` on the breeding digivolves

The T3 (P0) and T4 (P1) breeding digivolves carried `expect: { prompt:
breeding_action }`. A Lv.2 in an occupied breeding area can neither hatch nor
move, so neither engine asks a breeding decision that turn — the turn opens on
`main_phase` (the confirmed `P-180-effect1.yaml` convention). Our sim-side
check is loose there; DCGO asserts `expect:` strictly. Fixed to `main_phase`
in both files.

### `effect#3` — a missing `<Raid>` row (removed by changing the board)

The draft promoted P1's Agumon on T6. The effect attack ("1 of your Digimon may
gain <Rush> and attack a player") is still an attack declaration, so
Dinobeemon's own `<Raid>` (`RaidSelfEffect`, `OnAllyAttack`) would have opened
an `OptionalSkill` on the unsuspended Agumon — a row the draft did not have.
P1 now keeps Agumon in the breeding area all line (a declined `move`), so the
declaration stacks nothing.

### Asserts

Both files now pin the shared pre-clause board; `effect1` also pins the
post-switch board (Agumon deleted, security untouched at 5).

## CARD-YAML FINDING — the whole [When Digivolving] is gated on `dna_origin` (`effect#3`)

`code/digimon-engine/cards/bt16/BT16-077.yaml` puts `condition: { dna_origin:
true }` on the WHOLE clause. The printed text scopes "If DNA digivolving" to
the first sentence only; the official Bandai Q&A on the card
(`data/card_bundles/BT16-077.md`) says so in terms — "Even if this Digimon
didn't DNA digivolve, this card's [When Digivolving] effect can give 1 of your
Digimon <Rush> and that Digimon can attack" — and `BT16_077.cs` gates only the
trash play on `IsJogress(hashtable)`, running the Rush/attack half
unconditionally. So on an ordinary digivolution our engine opens nothing while
DCGO asks `OptionalSkill` (`isOptional: TRUE`) → `SelectPermanentEffect`
(`canNoSelect: true`) → `SelectAttackEffect` (`SetCanNotSelectNotAttack`,
players only). `BT16-077-effect3.yaml` scripts DCGO's three rows (they answer
no live prompt on our side and ride the wire) and is PREDICTED to read
`diverged` right after the digivolve (DCGO: Dinobeemon suspended, P1 security
4; ours: unsuspended, 5). The fix is to move the `dna_origin` gate onto the
trash-play steps only. Not applied here (exam stage). The same clause's DCGO
decline is crossed by `effect1` (`OptionalSkill` "no", wire-only row).

Also noted: the YAML's `alt_paths` author only the `purple Lv.4 / 4` circle;
the official DB and `data/card_overrides.json` print `Purple Lv.4 / 4` AND
`Red Lv.4 / 4`. Both lines here digivolve from a purple Lv.4, so the red
circle is un-probed.

## Prompt shapes (re-derived from the C#)

- `<Raid>`: `OptionalSkill` + `SelectPermanentEffect` (`canNoSelect: true`);
  ours one declinable pick → the fold (`expect: OptionalSkill` on the row).
- [When Digivolving]: `SetUpActivateClass(CanActivateCondition, …, -1, TRUE,
  …)` — the `OptionalSkill` is asked on EVERY digivolution into Dinobeemon
  (`CanActivateCondition` is only `IsExistOnBattleArea`).
- Both Dinobeemon circles cost 4, so a purple Lv.4 base opens no
  `SelectCountEffect`.
- Slot hygiene: one Digimon per seat in effect1/effect3; see above for
  effect2/inherited0.

## 2026-09-20 — the DNA block is gone (`G-TOOLING-EXAM-NO-DNA-VERB` RESOLVED)

The measured limit this file cites — no DNA verb on our wire, and DCGO's
`InputDriver.BuildMainPhaseAction` refusing the `DNA_DIGIVOLVE` range — was
closed by the close-out job `three-musketeers-2`:
`do: { dna: { card: <ID>, materials: [field.N, field.M] } }`
(`exam/scenario.rs` + `exam/lower.rs`), a `LoweredStep::DnaDeclaration` that
carries both materials' identities on ONE wire row, and DCGO `fc67f9ae6`
(player `D:/dcgo-build/scripted-v16`, preflight GO). See
`qa/dcgo-exams/BT8/NOTES-BT8-084.md` for the full shape and
`BT8-084-effect0.yaml` for a worked line.

So `BT16-077#effect#0` and the "if DNA digivolving" rider in `#effect#3` are no
longer `unreachable` for a tooling reason — they are `unmeasured` until a line
is authored and run. This note's `<Partition>` tuck route is unaffected.

## 2026-09-21 — `effect#0` AUTHORED; the `unreachable` above is RETRACTED

`BT16-077-effect0.yaml` now exists, lowers and asserts clean (14 checks / 0
failed, decks `tm-purple-line` / `tm-quiet-red` from the existing
`tm_bt16_077_pool.json` — no new book). The clause is **`unmeasured`**, not
`unreachable`: the `G-TOOLING-EXAM-NO-DNA-VERB` reason recorded in the
`effect#0` section above was closed on 2026-09-20 (the `dna:` verb plus DCGO
`fc67f9ae6`, player build `D:/dcgo-build/scripted-v16`) and the section
immediately preceding this one already said so. The table near the top of this
file still prints the stale `effect#0 | — | — | unreachable` row; the live
status is:

| Clause | Scenario | Sim-only | Status |
|---|---|---|---|
| `effect#0` | `BT16-077-effect0.yaml` (NEW, 2026-09-21) | lowers, asserts pass | `unmeasured` — awaiting the oracle |

**The line.** Youkomon ST6-07 (purple Lv.4, DP 6000, cost 5) is hard-played on
T3 and Birdramon ST1-05 (red Lv.4, DP 5000, cost 4) on T5 — both VANILLA, so
the declaration measures nothing but itself — and T7 declares
`dna: { card: BT16-077, materials: [field.0, field.1] }` for Cost 0. p0 stays
on memory 3 and keeps the turn, which is what makes "digivolve unsuspended"
readable on a Lv.5 that has just digivolved.

**STACKING ORDER is asserted, and the rule fixes it.** `general_rule.pdf`
**8-2-2-2** (p.17): the cards "are placed in order from top to bottom so that
the Digimon shown on the LEFT side of [DNA Digivolution] goes on top", and the
player chooses the order ONLY when the requirement names numbers or multiples
of a card. "Purple Lv.4 + red Lv.4" names neither, so the purple material goes
on top and there is no choice to expose (no no-approximations exposure gap
here). Measured: our engine produces `sources: [ST1-05, ST6-07]` — bottom-first,
i.e. red under purple — which is exactly 8-2-2-2. (The first draft of the file
asserted the opposite order and failed; the rule, not the draft, is right.)
DCGO agrees by construction — `JogressEvoRootsFrameIDs` is built from the
declaration's element order and `BT16_077.cs`'s `elements[0]` is
`PermanentCondition1` = purple Lv.4.

**Prompt shape.** The DNA condition is an `AddJogressConditionClass` with
`SetNotShowUI(true)` and `JogressCondition(elements, 0)`: DCGO asks NOTHING for
it, the whole declaration being one `PlayCardAction`. Ours parks a
`SelectionKind::Material` prompt per material, which rides one `SimOnlySelect`
row (zero wire rows). The [When Digivolving] then fires as on any digivolution
(`SetUpActivateClass(CanActivateCondition, …, -1, TRUE, …)` → one
`OptionalSkill`); this line **declines** it, so neither the `IsJogress`-gated
trash play nor the Rush/attack half opens on either wire — both belong to
`BT16-077-effect3.yaml`. The trash-play branch could not open on an accept
either: p0's trash is empty all line and DCGO skips the `SelectCardEffect`
when `HasMatchConditionOwnersCardInTrash` is false.

**Slot hygiene.** The only multi-Digimon main-phase action is the DNA
declaration, and `materials:` is resolved to top-card ids before the action is
applied, so it rides the wire as identities (`exam/adapter.rs` ~470-500;
`InputDriver.FindFrameByTopCardId`) — not as slots.

**Not duplicated here:** `effect#2` / `inherited#0` (`<Partition>`) were
re-diffed CLEAN by the engine-gap stage that closed
`G-ENGINE-PARTITION-SLOT-ENFORCEMENT-DEFERRED` and keep their `confirmed`
verdicts; this file touches neither.

**5 clauses: `effect#0` now authored and `unmeasured`, 0 unreachable.**

## 2026-09-21 — two leftovers the `effect#0` section above does not cover

**The `#effect#3` DNA RIDER is reachable now, and is still `unmeasured`.** The
first sentence of `#effect#3` ("If DNA digivolving, you may play 1 level 5 or
lower [Free] Digimon from your trash") was `unreachable` for the same
`G-TOOLING-EXAM-NO-DNA-VERB` reason as `#effect#0` and is not any more. It is a
SUB-BRANCH inside a clause id that already carries a scenario
(`BT16-077-effect3.yaml`, which rides the ORDINARY purple circle and reads the
ungated Rush/attack half), and the exam's file naming is one scenario per
clause id, so no second file was authored for it here. Reaching it needs the
`dna:` line of `BT16-077-effect0.yaml` PLUS a `[Free]`-attribute Lv.5-or-lower
Digimon in p0's TRASH at the declaration — `tm-purple-line` cannot produce one
without a new deck, and `BT16-077-effect0.yaml` deliberately declines the
`[When Digivolving]` so its own reading stays the DNA clause alone. Recorded
so the next dispatch does not re-derive it.

**`BT20-076-dna-rider` is not a thing.** The close-out's exam-verb stage
reported that string in its `newly_reachable` list. There is no BT20-076 exam
scenario, no `NOTES-BT20-076.md`, and BT20-076 is not in the Three Musketeers
clause denominator at all — `grep -rn BT20-076 qa/dcgo-exams/` matches only
`BT20-020-*.yaml`, where Imperialdramon: Dragon Mode is deck ballast and a
name-match target, never an exam subject. Read as the DNA rider that actually
exists in this pool, it is the `#effect#3` rider documented immediately above.

## 2026-09-21 (close-out `three-musketeers-2`) — `effect#0` MEASURED: verdict **confirmed**; card CLOSED

The DNA clause drained `completed` on oracle build `scripted-v16` (DCGO `fc67f9ae6`)
and diffed **CLEAN**:

| Clause | Sidecar | Diff | Verdict |
|---|---|---|---|
| `BT16-077#effect#0` | `20260921T041912Z_234cef51` | CLEAN, compared 13 of 13 ours / 13 dcgo | **confirmed** |

**It first reported a DIVERGENCE that was pure index offset, from a harness bug.**
The first run read `DIVERGED at step 12 … turn: ours=6 dcgo=5, phase: ours=Breeding
dcgo=Main, memory: ours=-3 dcgo=3` — a whole scenario step of skew, which is the
signature of an offset rather than of a rules disagreement. Cause:
`ScenarioAdapter::dcgo_wire_rows_per_step` mapped wire-row counts 1:1 over the
LOWERED entries while the differ indexes them by SCENARIO step; this line's `dna:`
step lowers to `DnaDeclaration` + `SimOnlySelect`, so every row after it was compared
against our projection one step late and the last row fell off the end. Fixed in this
stage — `G-TOOLING-EXAM-PAIRING-INDEXED-BY-LOWERED-ENTRY` in
`docs/RUST_ENGINE_GAPS.md`. The `confirmed` above is the re-diff against the SAME
sidecar, with all 13 rows compared on both sides.

If a future exam line reports a lead whose `turn` / `phase` / `memory` all move
together by exactly one step, suspect pairing before suspecting the engine.

**5 clauses: 5 confirmed, 0 diverged, 0 unreachable, 0 unavailable, 0 unmeasured.**
