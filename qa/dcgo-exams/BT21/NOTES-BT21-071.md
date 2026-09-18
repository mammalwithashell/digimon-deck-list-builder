# NOTES — Scopemon (BT21-071): clauses no scripted line can reach

Denominator (`clause_coverage.extract --card-ids BT21-071`): **5 clauses** —
`effect#0`, `effect#1`, `effect#2`, `inherited#0`, `inherited#1`.

Scenarios authored: `BT21-071-effect0.yaml` (alt-digivolve condition; needs
the book `tm_bt21_071_pool.json` for its black Digi-Egg) and
`BT21-071-effect1.yaml` ([On Play] tuck → gain 1 memory).

The remaining three clauses are the card's **DigiLink** half and are
`unreachable` for one measured reason: **the exam's scripted-step vocabulary
has no verb that declares a link, on either engine's side of the wire.**

| Clause id | Text | Verdict | Measured reason |
|---|---|---|---|
| `BT21-071#inherited#0` | `<Link>` [Appmon] trait: Cost 2 (Plug this card from the hand or battle area sideways into the specified Digimon…) | `unreachable` | No link verb (see below). |
| `BT21-071#inherited#1` | [When Linking] `<Draw 2>` and trash 2 cards in your hand. | `unreachable` | Fires only after a link is declared; no link verb. |
| `BT21-071#effect#2` | +3000 DP (the link-box DP grant) | `unreachable` | Observable only on a host while Scopemon is linked; no link verb. |

## Why "no link verb" is a measured limit, not a guess

1. **Scenario vocabulary.** `code/tools/dcgo-harness/src/exam/scenario.rs`
   line 208 fixes the `do:` verbs to exactly
   `hatch, pass, move, play, digivolve, attack, main, select`. There is no
   `link`.
2. **`play:` cannot stand in for a hand-link.** `lower.rs::matches_intent`
   accepts a `play:` step only for `ActionKind::Play`. Our engine exposes
   no hand-initiated *Digimon* link action at all: `action/mask.rs` emits
   link only on the per-permanent `FIELD_EFFECT` range at sub-slot
   `FIELD_EFFECT_SLOT_FOR_LINK = 3` (`space.rs:123`, `mask.rs:386-394`;
   `game_actions/link.rs::activate_field_link`). The only hand-side link
   path (`game_actions/mod.rs` ~1417, `OptionSubtype::Link`) is for
   *Option*-type Link cards, which Scopemon is not.
3. **`main:` cannot stand in for a field-link either.** `main: { on:
   field.N }` lowers to whatever `FIELD_EFFECT` bit the slot offers, but
   DCGO's harness refuses it: `Assets/Scripts/Script/Harness/InputDriver.cs`
   lines 380-386 map the `FIELD_EFFECT` range to
   `ActivatePermanentAction(slot, 0)` and reject any sub-slot other than
   `FIELD_EFFECT_SLOT_FOR_MAIN`. A lowered link id would abort the job on
   the DCGO side before any prompt is asked, so no oracle answer is
   obtainable through that route.

Until the scenario format gains a link verb (and DCGO's `InputDriver`
accepts the link sub-slot), these three clauses stay `unreachable` with this
reason — never a silent skip, never `confirmed`. This is a tooling gap in the
exam surface, not a finding about Scopemon's behaviour.

**Update 2026-09-17 — the gap above is closed; the three clauses are now
reachable and simply `unmeasured` until their lines are authored.** The
format gained `link: { card: BT21-071, from: field.N | hand.N }`, followed by
`select: { targets: [own.field.N] }` for the host (both engines park that
pick; `docs/DCGO_EXAM.md` "`link:`"). Points 2 and 3 above no longer hold:
our engine now exposes the from-hand Digimon link on the hand slot's
`HAND_EFFECT` bit (`G-ENGINE-DIGIMON-LINK-FROM-HAND`, resolved), and DCGO's
`InputDriver` (base DCGO `c98bab2a4`, player `D:/dcgo-build/scripted-v15`)
dispatches both the FIELD_EFFECT link sub-slot and the hand bit to the
`LinkEffect` `ActivateClass`. The three lines still need a book that puts an
`[Appmon]` host beside Scopemon (e.g. BT21-009 Gatchmon) and were not
authored in the tooling stage.

DCGO does script the link half (`BT21_071.cs`: `CardEffectFactory.LinkEffect`,
`AddSelfLinkConditionStaticEffect(HasAppmonTraits, linkCost: 2)`, and the
`WhenLinked` ActivateClass with `isOptional: false` → mandatory `DrawClass(2)`
then `SelectHandEffect(mode: Discard, maxCount: min(2, hand), canNoSelect:
false)`), so the card is **not** `unavailable`; the oracle exists, the wire
cannot reach it.

## Update 2026-09-18 — the three Link clauses are authored; nothing is `unreachable`

**Current status: 5 clauses, 5 scenarios, 0 unreachable, 0 unavailable.** The
table at the top of this file is history (it predates the `link:` verb). All
five files lower sim-only against the current engine/harness
(`effect0` with `tm_bt21_071_pool.json`, the rest with
`../EX7/three_musketeers_pool.json`); `effect#0` / `effect#1` were re-lowered
unchanged after the OnAddDigivolutionCards / OnUseOption engine work.

| Clause id | Scenario | Line |
|---|---|---|
| `BT21-071#inherited#0` | `BT21-071-inherited0.yaml` | `<Link>` declared from the **hand**: Scopemon B into Scopemon A, cost 2 (3 -> 1). |
| `BT21-071#inherited#1` | `BT21-071-inherited1.yaml` | `<Link>` declared from the **battle area** (`from: field.1`), then the mandatory `<Draw 2>` + trash 2 over the whole post-draw hand. |
| `BT21-071#effect#2` | `BT21-071-effect2.yaml` | Host DP 4000 -> 7000, witnessed by a security battle against Birdramon ST1-05 (5000) that a 4000-DP Scopemon would lose. |

Authoring decisions the oracle run should know about:

- **The host is another Scopemon.** Scopemon's own traits are Sup. / Appmon /
  Tool / Monitoring, so the shared `three-musketeers` deck already holds an
  `[Appmon]` host and no per-card book was needed (the 2026-09-17 note's
  "e.g. BT21-009 Gatchmon" turned out unnecessary).
- **The linked Scopemon is DRAWN after the host is played**, never held with
  it. A Scopemon (or Satellamon) in hand is an `[Appmon]`-trait card, which
  would make the host's `[On Play]` tuck prompt real on both sides. With only
  `[TS]` Tamers in hand the prompt is vacuous: DCGO still asks its
  `OptionalSkill` (`CanActivateConditionShared` only needs a non-empty hand),
  ours parks nothing -> a `dcgo_only` decline, the `effect0` shape.
- **`[When Linking]` discard is 1 DCGO prompt vs 2 of ours.** DCGO:
  `SelectHandEffect(Mode.Discard, maxCount: min(2, hand), canNoSelect: false,
  canEndNotMax: false)`; ours: two sequential `select_hand` picks. One
  two-card `cards:` answer feeds both (the `BT19-075-effect1.yaml`
  convention) and is one wire row; `expect.count: 2` is DCGO's `maxCount`.
- **No OptionalSkill anywhere in the link sequence.** `LinkEffect` is built
  `isOptional: true` but is a *declared* (`OnDeclaration`) effect, and the
  `WhenLinked` `ActivateClass` is `isOptional: false`; with one trigger there
  is no `MultipleSkills` either. So the wire is: `main_phase` (the link bit)
  -> `SelectPermanentEffect` (host; asked even with one candidate) ->
  `SelectHandEffect`.
- **Not projected:** the linked card itself. The state projection has no
  link-card field, so "B is attached to A" is witnessed indirectly (B left the
  hand / the field, A's DP carries the link box, the `[When Linking]` fired).

## Update 2026-09-18 (oracle) — 5/5 confirmed; the Link declaration DOES ask an OptionalSkill

The first oracle run aborted all three Link lines ("expected prompt
'SelectPermanentEffect' but DCGO asked 'OptionalSkill'"). The "No OptionalSkill
anywhere in the link sequence" bullet above was wrong: `LinkEffect` is
`SetUpActivateClass(null, ActivateCoroutine, -1, true, ...)`
(`CardEffectFactory/KeyWordEffects/Link.cs`), and DCGO confirms a declared
optional effect with a yes/no BEFORE its coroutine runs. Our engine treats the
declaration as the decision. A prompt-existence difference, not a rules finding:
each line gained one `select: { yes: true, dcgo_only: true }` row after the
`link:` step (asserts re-indexed) and the re-run diffed CLEAN. Every future
`link:` line needs the same row.
