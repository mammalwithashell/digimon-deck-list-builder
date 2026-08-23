# Toho Braves — DCGO card-clause exam report

**Date:** 2026-08-23 · **Campaign:** `toho-braves-exam` · **Oracle:** DCGO player
`scripted-v8` (`dcgo_commit 617cf381b`) · **Store:**
`qa/qa-reports/dcgo_exam_verdicts.json` · **Scenarios:** 135 under `qa/dcgo-exams/`
· **Denominator correction:** `qa/qa-reports/clause-denominator-correction.md`

## The denominator, always first

**166 clauses across the 42-card tournament pool: 71 confirmed · 11 diverged ·
23 unreachable · 61 unmeasured.**

**82 clauses — 49% — have actually been run in both engines** and compared
step-by-step. The 18-card competitive core (in ≥33 of 45 tournament lists) is at
**44 of 74 = 59%**.

Never read this as "Toho Braves passed." Read it per clause: a `confirmed` is a
scripted line that ran identically in our engine and in DCGO under a full
per-step state diff; everything else is exactly as unproven as it says.

### The denominator itself was wrong, and is now corrected

An earlier version of this report said **239 clauses**. That number was inflated
by **28.9%** with slots that are not printed clauses:

| Cause | Removed | How it was confirmed |
|---|---|---|
| Phantom `#security#0` slots | 27 | No [Security] box on the card face, **and** zero `SecuritySkill` in DCGO's C#, **and** an empty cards.json field — three independent sources |
| Splitter fragments | 36 | One printed sentence cut into `"Activate this card's"` + `"effect."`; one clause whose entire text was `"."` |
| MediaWiki scrape residue | 6 | Clause text literally `\|applinkdp =` |

The security zone alone fell from **31 slots to 4**. So most of the "structurally
unmeasurable security clauses" this report used to carry as permanently-unreached
simply **did not exist**. Fixed in `code/tools/clause_coverage/`; the correction is
measured card-by-card in `clause-denominator-correction.md`.

| Card | Name | In lists | Clauses | confirmed | diverged | unreachable | unmeasured |
|---|---|---|---|---|---|---|---|
| EX12-076 | Susanoomon | 45/45 | 6 |  |  | 4 | 2 |
| EX12-004 | Onibimon | 44/45 | 2 | 1 |  |  | 1 |
| EX12-009 | Wankomon | 44/45 | 3 | 2 |  | 1 |  |
| EX12-020 | Gasamon | 44/45 | 3 | 2 |  |  | 1 |
| EX12-026 | Shellmon | 44/45 | 4 | 2 |  |  | 2 |
| EX12-031 | MarineBullmon | 44/45 | 4 | 1 |  | 3 |  |
| EX12-046 | Shishimamon | 44/45 | 4 | 2 | 1 | 1 |  |
| EX12-047 | Amaterasumon | 44/45 | 6 | 3 |  | 3 |  |
| EX12-061 | Hanimon | 44/45 | 4 | 4 |  |  |  |
| EX12-062 | Kokeshimon | 44/45 | 4 | 1 |  | 3 |  |
| EX12-063 | Karakurumon | 44/45 | 4 | 2 |  | 2 |  |
| EX12-065 | Kaguyamon | 44/45 | 5 | 4 |  |  | 1 |
| EX12-070 | Sanmyojin Arrival | 44/45 | 4 | 4 |  |  |  |
| EX12-011 | Seasarmon | 42/45 | 4 | 4 |  |  |  |
| EX12-036 | Ryugumon | 42/45 | 6 | 2 |  | 4 |  |
| EX12-074 | Genshi Continent & Ashin | 40/45 | 4 | 3 |  | 1 |  |
| EX1-066 | Analog Youth | 34/45 | 3 | 3 |  |  |  |
| EX12-075 | Kunlun's Imperial Decree | 33/45 | 4 | 3 |  | 1 |  |
| EX12-043 | Hakubamon | 7/45 | 3 | 2 | 1 |  |  |
| BT8-097 | Crimson Blaze | 3/45 | 3 | 3 |  |  |  |
| P-130 | Lui Ohwada | 3/45 | 3 | 1 | 2 |  |  |
| EX12-025 | Gawappamon | 2/45 | 4 | 4 |  |  |  |
| ST16-14 | Matt Ishida | 2/45 | 3 | 3 |  |  |  |
| ST19-14 | Arisa Kinosaki | 2/45 | 3 | 1 |  |  | 2 |
| BT11-089 | Akiho Rindou | 1/45 | 3 | 1 |  |  | 2 |
| BT20-037 | Chaosmon: Valdur Arm | 1/45 | 4 |  |  |  | 4 |
| BT8-084 | Kimeramon | 1/45 | 3 |  |  |  | 3 |
| EX12-002 | Mococomon | 1/45 | 3 |  | 2 |  | 1 |
| EX12-006 | Kakamon | 1/45 | 3 | 3 |  |  |  |
| EX12-012 | Apemon | 1/45 | 4 | 4 |  |  |  |
| EX12-015 | Gokuumon | 1/45 | 4 | 2 |  |  | 2 |
| EX12-019 | Nezhamon | 1/45 | 8 | 1 | 5 |  | 2 |
| EX12-022 | Kamemon | 1/45 | 3 | 2 |  |  | 1 |
| EX12-029 | Sagomon | 1/45 | 4 |  |  |  | 4 |
| EX12-034 | Erlangmon | 1/45 | 4 |  |  |  | 4 |
| EX12-039 | Takinmon | 1/45 | 3 |  |  |  | 3 |
| EX12-045 | Sanzomon | 1/45 | 4 |  |  |  | 4 |
| EX12-048 | SeitenGokuumon | 1/45 | 8 |  |  |  | 8 |
| EX12-056 | Cho-Hakkaimon | 1/45 | 5 |  |  |  | 5 |
| EX12-057 | Takutoumon | 1/45 | 3 |  |  |  | 3 |
| EX12-071 | Saneiketsu Invitation | 1/45 | 4 | 1 |  |  | 3 |
| EX4-074 | ShineGreymon: Ruin Mode | 1/45 | 3 |  |  |  | 3 |

## What `confirmed` means

A hand-authored scenario drives BOTH engines through the same legal line from
game start — same stacked deck, same seed, same actor sequence, every prompt
asserted before it is answered — and a normalized per-step state diff (board,
effective DP, suspension, hands, trash, security count, memory) came back CLEAN.
Selections are answered by **card identity** on both sides, so neither engine's
internal indices leak onto the wire.

It is scoped: a clause confirmed on one line is confirmed *on that line*. A
clause reached via a hard-cast is not thereby verified for its digivolve spine or
its alternate costs. That is inherent to exam-style verification, and it is why
the per-card `cards_behavioral` tests remain complementary rather than redundant.

## The 11 diverged — findings to triage

`diverged` means both engines ran the line end-to-end and disagreed about the
state. `general_rule.pdf` outranks DCGO; neither engine is presumed right.

Earlier in the campaign 12 clauses read as diverged and **11 of those turned out
to be representation, not defects** — our engine parks an `EndOfTurnAction` phase
where DCGO stays on `Main`, and §6-6-2 ("the current phase will continue until all
processing has been resolved") says the rules define no end-of-turn phase at all.
Normalized as a *pair* rule, with tests pinning that a genuine phase mismatch
still diverges. Expect some of the current 11 to dissolve the same way — and
expect some not to. `EX12-046#effect#2` (`p1.trash: ours=[EX12-062] dcgo=[]`) has
survived every pass so far and is a real finding.

## Engine defects this campaign found and fixed

Every one was found by *authoring a scenario*, and most produce no failing unit
test — which is the point of the exercise.

| Defect | Ruling |
|---|---|
| Granted `<Execute>` never fired at all | §16-37-3 + §15-9-2-2 — our rule-17 violation; 13 clauses affected |
| DSL `grant_keyword` never reached the would-be-deleted window (`<Barrier>`/`<Evade>` dead from grants) | printed-keyword path worked, grant path did not |
| Effect-initiated digivolve ignored alt trait circles | consulted printed `evo_costs` only, never the registry alt paths the mask path honors |
| Declining one optional replacement consumed the event | §15-8-5-4 — only the *activated* effect cannot re-activate |
| Dead cards' `on_ally_played` observers fired from the trash | §15-14-3-1 — trash triggering is exclusive to `{Trash}`-icon effects |
| `EX12-031` `<Decode>` over-asked on another permanent's leave | §16-35-1 scopes it to the Digimon *with* the effect |
| A stale deletion-cause **test** (not the engine) | §13-1-7-3-1 + §14-2-2 — a security-check deletion *is* a Battle deletion |

Plus two data fixes: EX12-036/EX12-047 print `[TB]` (and EX12-036 Rule-grants
`[Aquatic]`) but `cards.json` had dropped them — repaired durably in
`card_overrides.json`.

## The 23 unreachable, and the 61 unmeasured

`unreachable` carries a named cause per clause in the store. The surviving
families: MultipleSkills value-space edges, prompt-shape asymmetries
(`SelectCardEffect` vs `main_phase`, `OptionalSkill` vs `MultipleSkills`), 1-card
`OrderedPermutation` rows DCGO auto-places, Material indexes-payload prompts
(out of `select:` scope by design), and actor-parity on long lines.

The 61 unmeasured are concentrated in low-play support cards — 1-ofs and 2-ofs
like Chaosmon, Akiho Rindou, ShineGreymon: Ruin Mode. Nothing blocks them
technically; they are unauthored, and each is worth far less per clause than a
core-card clause.

## Tooling this campaign produced (exam-general, not Toho-specific)

- `select:` steps end to end — five symbolic forms, identities on the wire, both
  engines resolving against their own candidate lists.
- The `stack[5..9] → reversed security stack` mapping, which turned the entire
  security family from "structurally unmeasurable" into ordinary authoring.
- `move` verb; `--verdicts` store with clause-text drift guards and orphan
  refusal; `--emit-job`; phase normalization for selection and combat-interrupt
  windows.
- A resumable oracle loop with a single-instance lock and **progress-based** stall
  detection — the heartbeat ticks from the poll loop and stays fresh through a
  hung game, which once hid a 62-minute hang.
- Adversarial pre-Unity review of every authored scenario against DCGO's C#.

## Reproducing

```bash
# CI-safe, no Unity: re-check every scenario's assertions
dcgo-harness exam --scenario qa/dcgo-exams/ --sim-only --cards-json data/cards.json

# oracle pass (needs the DCGO player build) -- see docs/DCGO_EXAM.md
```
