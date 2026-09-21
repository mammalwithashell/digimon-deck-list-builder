# NOTES — Tidal Stream (BT24-091)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT24-091`):
**6 clauses** — `effect#0` (ignore colour requirements while you have a [TS]
Digimon or Tamer), `effect#1` (`[Security] Activate this card's [Main]
effects`), `effect#2` (the [Main] body), `effect#3` (Link DP `DP+2000`),
`effect#4` (Link Condition `<Link> [TS] trait: Cost 3`), `effect#5` (Link
Effect `[When Attacking] [Once Per Turn] Return 1 of your opponent's lowest
level Digimon to the hand`).
DCGO script: `BT24/Blue/BT24_091.cs` exists — the card is **not** `unavailable`.

**Status: 6 clauses — 5 scenarios, 1 `unreachable` (`effect#4`).** Book:
`qa/dcgo-exams/BT24/tm_bt24_091_pool.json` (one deck, `tm-ts-line`, both seats).

| Clause | Scenario | Sim-only | Expected at the oracle |
|---|---|---|---|
| `effect#0` | `BT24-091-effect0.yaml` (NEW) | lowers, asserts pass | clean |
| `effect#1` | `BT24-091-effect1.yaml` (REPAIRED) | lowers, asserts pass | clean |
| `effect#2` | `BT24-091-effect2.yaml` (RE-AUTHORED, from hand) | lowers, asserts pass | host DP only (the `effect#3` finding) |
| `effect#3` | `BT24-091-effect3.yaml` (REPAIRED) | lowers, asserts pass | **predicted divergence — card YAML** |
| `effect#4` | — | — | **unreachable** (below) |
| `effect#5` | `BT24-091-effect5.yaml` (REPAIRED) | lowers, asserts pass | **predicted divergence — card YAML** |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

Four files (`effect1/2/3/5`) were on disk, uncommitted, all one [Security]
line, and all written BEFORE two fixes that changed what they measure:

1. `35958972b` closed `G-ENGINE-SECURITY-OPTION-LINK-TO-OWN-DIGIMON`: a
   security-flipped Plug-In Option now links on our side. The drafts'
   "PREDICTED DIVERGENCE: our engine silently skips the link and trashes the
   card" is obsolete — re-measured here, `p1.trash` is empty after the link.
2. `d5f23792f` added `select: { choice: "<label>", sim_only: true }`. The
   drafts had declared the whole FROM-HAND surface unreachable because no
   payload could answer our mode-select `EffectChoice` ("Play as a [Main]
   Option" / "Plug in via Link Requirements (Cost 3)"). `choice: "[Main]"`
   answers it with zero wire rows — DCGO's Play bit is `PlayCardAction`, the
   [Main] use, and asks nothing. So `effect#0` and `effect#2` are now measured
   from hand.

What each file is now:

- `effect#1` keeps the [Security] line but DECLINES the link, so the clause
  ("Activate this card's [Main] effects": bounce + unsuspend pick + link pick,
  all run for the DEFENDER on the attacker's turn) is measured clear of the
  Link box. Expected clean.
- `effect#2` is the [Main] used from hand with every sentence visible: two
  opposing Digimon of different levels (only the Lv.3 returns), a Cyclonemon
  that has just attacked (so the unsuspend changes the board), and the free
  link accepted (the Option leaves the hand for the link slot, memory moves by
  5 only).
- `effect#3` adds a NON-[TS] Digimon (Biyomon) to the linker's board so the
  link pick's `candidates: [BT25-024]` shows the printed Link condition gating
  even a free effect link (`CanLinkToTargetPermanent(permanent, false, true)`).
- `effect#5` is unchanged in shape; header and asserts rewritten to the
  current engine.

## `effect#4` — `unreachable`: a DECLARED hand `<Link>` of a Plug-In OPTION has no common wire

Measured 2026-09-18 on the current harness (`scripted-v15` vocabulary):

- The exam's `link: { card, from: hand }` lowers to that hand slot's
  `HAND_EFFECT` bit. Our mask emits that bit for an Appmon Link **Digimon**
  (`Game::hand_effect_slot_is_link`), but for a Plug-In **Option** the hand
  link is NOT a separate action: it is branch 1 of the mode-select
  `EffectChoice` that `Game::play_option_core` parks behind the ordinary PLAY
  bit. Probe: `link: { card: BT24-091, from: hand }` with a legal [TS] host on
  the board → `no legal action matches Link { … }`; the legal set holds only
  `0: Play Tidal Stream from hand 0`.
- The other way round cannot work either: a `play:` step lowers to the PLAY
  id, which DCGO's `InputDriver` turns into `PlayCardAction` — the [Main] USE,
  never the link (DCGO's hand link is `ActivateCardAction` over the
  `LinkEffect` declaration). Choosing our branch 1 `sim_only` would make the
  two engines do different things with one wire row.
- There is no `dcgo_only` form for a main-phase ACTION (only for `select:`
  rows), so the line cannot be split per engine.

So "Cost 3" (paying 3 to declare the link) cannot be reached. Tooling gap
`G-TOOLING-EXAM-OPTION-HAND-LINK` (`docs/RUST_ENGINE_GAPS.md`): either the mask
grows a `HAND_EFFECT` link bit for Plug-In Options (mirroring the Digimon
path), or `link:` learns to lower to PLAY + the `EffectChoice` link branch on
our side while emitting DCGO's `ActivateCardAction`. The clause's OTHER half —
the `[TS] trait` host gate — IS witnessed, on the free effect link, by
`effect#3`'s candidate set. Never a silent skip, never `confirmed`.

## CARD-YAML FINDINGS — the printed Link box is not authored (`effect#3`, `effect#5`)

`code/digimon-engine/cards/bt24/BT24-091.yaml` ends at the `link_requirement`.
Neither half of the Link box exists:

1. **Link DP `DP+2000`** — already OPEN as
   `F-DATA-LINK-OPTION-PRINTED-LINK-BOX-NOT-AUTHORED` (BT24-091 is on its
   list; fixed the same day on BT25-093 / BT25-100 with a `scope: linked`
   `dp_modifier: 2000` aura). Re-measured here: the linked Lekismon reads 5000.
2. **Link Effect `[When Attacking] [Once Per Turn] Return 1 of your opponent's
   lowest level Digimon to the hand`** — NEW, not on that finding's list (it
   tracks Link DP only). There is no linked `when_attacking` clause at all, so
   a linked host attacks with no trigger: `BT24-091-effect5.yaml` step 11
   "select answered no live prompt". DCGO: region "Link ESS",
   `SetUpActivateClass(CanActivateCondition, …, 1, FALSE, …)` +
   `SetIsLinkedEffect(true)`, one mandatory `SelectPermanentEffect`
   (`Mode.Bounce`, `canNoSelect: false`) over `IsMinLevel` opposing Digimon.
   The fix is the `BT25-093.yaml` "Link ESS" idiom (mandatory — the text has no
   "you may").
   **RESOLVED 2026-09-19**: authored with that idiom; `BT24-091-effect5.yaml`
   re-diffed CLEAN 14/14 against the preserved sidecar, verdict CONFIRMED.

Both are appended to the finding in `docs/RUST_ENGINE_GAPS.md`. Not fixed here
(exam stage; the harness binary embeds the card pool and this stage does not
rebuild). The sim asserts on `effect2` / `effect3` / `effect5` pin everything
EXCEPT the linked host's DP and the bounce, so the three lines go clean the
moment the two clauses are authored.

## Prompt shapes (re-derived from the C#)

- [Main] / [Security] (one ActivateClass, `SetUpActivateClass(null, …, -1,
  FALSE, …)`; [Security] re-runs it via `AddActivateMainOptionSecurityEffect`):
  no `OptionalSkill`. Bounce = no prompt. Unsuspend = `SelectPermanentEffect`
  (`Mode.UnTap`, `maxCount 1`, `canNoSelect: FALSE`) over own battle-area [TS]
  Digimon, NOT filtered on being suspended, asked even with one candidate — and
  skipped entirely when nothing was returned. Link = `SelectPermanentEffect`
  (`Mode.Custom`, `maxCount 1`, `canNoSelect: TRUE`), no `OptionalSkill` in
  front of it (that gate belongs to a DECLARED `<Link>`, `Link.cs`
  `isOptional: true` — `NOTES-BT21-074.md`).
- Ignore-colour: `IgnoreColorConditionClass`, static, no prompt.
- Slot hygiene: every `select:` on a two-Digimon board here is an identity
  pick (`targets:` ride the wire as top-card ids); no `attack:` / `digivolve:`
  addresses a slot on a two-Digimon board.

## 2026-09-20 — the Option hand plug-in is now its OWN main-phase action

`G-TOOLING-EXAM-OPTION-HAND-LINK` is **closed**, and it was closed on the
ENGINE side — the option these notes preferred — so no DCGO change was needed
(close-out job `three-musketeers-2`).

**The rules reading.** `general_rule.pdf` (Ver.3.6) §6-5-1 lists the main-phase
actions, and **§6-5-1-3 "Use an Option Card From the Hand"** and **§6-5-1-4
"Linking a Card in the Hand or Battle Area"** are two SEPARATE entries; §10-1-1
says a card is linked from the hand "by paying the cost **as part of the main
phase actions**". So plugging a Plug-In Option in from hand is its own
declaration, not a mode of using the card. DCGO already drew that line:
`CardEffectFactory.LinkEffect` (`Link.cs:19-24`) accepts ANY `CardSource` with
a `linkCondition` that `IsExistOnHand`, so the declaration sits in the card's
`CanDeclareSkillList` and `InputDriver.FindHandDeclarableSkillIndex` reaches it
as an `ActivateCardAction` — never a `PlayCardAction`.

**What changed in the engine** (`G-ENGINE-OPTION-LINK-FROM-HAND`,
`docs/RUST_ENGINE_GAPS.md`): `hand_option_link_condition_targets` /
`hand_option_link_available` put the Plug-In Option's from-hand link on the
`HAND_EFFECT` bit, exactly where the Link DIGIMON's already lived
(`G-ENGINE-DIGIMON-LINK-FROM-HAND`, `cf8e2519f`); `activate_hand_link` routes
an Option through `play_option_core` in `OptionPlayMode::Link`; and
`option_legal_play_modes` drops the Link mode for a HAND source so the PLAY bit
means only §6-5-1-3 and the declaration is not exposed twice. A
`[Security]`-flipped Option keeps its Link mode — it has no main-phase
declaration to make.

**For this file**: the clause is reachable now. Author it as

```yaml
- actor: 0
  do: { link: { card: <THIS CARD>, from: hand } }
- actor: 0
  do: { select: { targets: [own.field.N] } }
  expect: { prompt: SelectPermanentEffect, count: 1 }
```

— the same two-step shape every other `link:` line uses, on the same wire
integer on both engines. It needs no new DCGO build (`scripted-v15` already
dispatches a `HAND_EFFECT` bit to the hand card's link declaration); the
scenario itself has not been written yet, so the clause is `unmeasured`, not
`unreachable`.

## 2026-09-21 — `effect#4` AUTHORED; the `unreachable` above is RETRACTED

`BT24-091-effect4.yaml` now exists, lowers and asserts clean (13 checks / 0
failed, book `tm_bt24_091_pool.json`). The clause's status is **`unmeasured`**,
not `unreachable`: the tooling reason recorded in the `G-TOOLING-EXAM-OPTION-HAND-LINK`
section above was closed on the engine side (`G-ENGINE-OPTION-LINK-FROM-HAND`,
commit `7e30343d4`) and the section immediately preceding this one already
retracted it — this entry is the line that reason was blocking.

**Status table correction.** The table near the top of this file still reads
`effect#4 | — | — | unreachable`; the live status is:

| Clause | Scenario | Sim-only | Status |
|---|---|---|---|
| `effect#4` | `BT24-091-effect4.yaml` (NEW, 2026-09-21) | lowers, asserts pass | `unmeasured` — awaiting the oracle |

**What the line pins, and why it is not a duplicate of `effect#3`.**
`effect#3`'s line reaches the link through the card's own [Main]/[Security]
"…link this card … **without paying the cost**" branch, so it can witness the
host gate but never the cost. This line makes the DECLARATION instead — its own
main-phase action per `general_rule.pdf` 6-5-1-4 (distinct from 6-5-1-3 "Use an
Option Card From the Hand") with 10-1-1's "paying the cost as part of the main
phase actions" — from memory 3 down to 0, and asserts that the Option left the
HAND (in neither hand nor trash afterwards) for Lekismon's link slot.

**Host gate witness.** p0 fields Agumon ST1-03 (red, no [TS]) BEFORE Lekismon
BT25-024, so the host pick asserts `candidates: [BT25-024]` with a non-[TS]
Digimon sitting on the same board — DCGO gates it in `Link.cs`'s
`CanSelectPermanentCondition` via the `linkCondition.digimonCondition` that
`AddSelfLinkConditionStaticEffect(PermanentCondition, linkCost: 3, …)` installs
in BT24_091.cs's "Link Condition" region (`IsDigimon && TopCard.HasTSTraits`);
ours by the `kind: link_requirement` filter in `cards/bt24/BT24-091.yaml`.

**Prompt shape.** `CardEffectFactory.LinkEffect` is
`SetUpActivateClass(null, ActivateCoroutine, -1, TRUE, …)` (`Link.cs:27`), so
DCGO asks an `OptionalSkill` yes/no BEFORE the host pick while our engine treats
the declaration itself as the decision — that row is `dcgo_only`, the same fold
`../BT21/BT21-071-effect2.yaml` carries and which was measured against the
oracle there. The suggested two-step shape written into the previous section
(`link:` then `select:`) was therefore **one row short**; the committed file has
all three.

**Slot hygiene.** No main-phase action addresses a `field.N` on a
multi-Digimon board (both plays come from hand; the link declaration names a
HAND card). The one two-Digimon reference is the host `select:`, which rides
the wire as the target's card identity.

**6 clauses: 5 with an oracle verdict, `effect#4` now authored and
`unmeasured` — 0 unreachable.**

## 2026-09-21 (close-out `three-musketeers-2`) — `effect#4` MEASURED: **diverged** (engine finding), 0 unmeasured left

The newly authored `effect#4` line drained `completed` on oracle build `scripted-v16`
(DCGO `fc67f9ae6`). The `unmeasured` above is resolved; the clause is **diverged**.

| Clause | Sidecar | Diff | Verdict |
|---|---|---|---|
| `BT24-091#effect#4` | `20260921T042039Z_c3e0a78e` | DIVERGED at step 11, compared 13 of 14 ours / 14 dcgo (1 sim-only + 1 DCGO intermediate row) | **diverged** |

**The one divergence row**, at the HOST pick (`--all-diffs`: LEAD only, no
downstream):

- `memory: ours=0 dcgo=3`
- `p0.hand: ours=[ST1-05, ST1-08, ST1-08, ST1-08, ST1-10]` /
  `dcgo=[BT24-091, ST1-05, ST1-08, ST1-08, ST1-08, ST1-10]`

Ours has already paid the link cost and removed the Option from hand when the host is
chosen; DCGO has not.

**The PDF backs DCGO.** `general_rule.pdf` §10-1-3 (p.19) spells the link procedure
out in order: **10-1-3-1** declare the link and reveal the card, choose 1 link
requirement, *then* choose 1 of your Digimon that meets it → **10-1-3-2** "The
specified link cost is paid" → **10-1-3-3** the card is plugged in sideways. Payment
is step 2 of 3, after the host. §10-1-2-3 makes the window observable rather than
cosmetic: an immediate-type effect triggering on a link "will trigger immediately
after the card is revealed and the Digimon to be linked is chosen", i.e. between the
host choice and the payment.

**What the clause itself still establishes.** Steps 12-13 agree on both wires: memory
0, the Option off the hand and in Lekismon BT25-024's link slot, cost 3 paid, and the
host gate held (Agumon ST1-03, red and non-`[TS]`, sat on the same board and was never
offered). So `<Link> [TS] trait: Cost 3` **from the hand** is right on our side; only
the ORDER of the payment is wrong.

**Not fixed here** (verdict-recording stage). Routed to the EXISTING
`G-ENGINE-OPTION-HAND-LINK-COST-TIMING` section of `docs/RUST_ENGINE_GAPS.md`, where
this clause is now a third driver beside `BT25-093#effect#4` and `BT25-100#effect#5` —
three independent Plug-In Options producing the identical single row, which makes it a
property of `Game::activate_hand_link` → `play_option_core(.., OptionPlayMode::Link,
OptionCostPolicy::Pay)` (`code/digimon-engine/src/game_actions/link.rs:197-217`) rather
than of any one card. Our **Digimon**-link path is already correct.

**6 clauses: 5 confirmed, 1 diverged, 0 unreachable, 0 unavailable, 0 unmeasured.**

## 2026-09-21 (close-out `three-musketeers-2`, triage stage) — `effect#4` RE-MEASURED: **confirmed**

The divergence above was a genuine engine finding and it has been FIXED — not by this
stage, but by `9af21698c` (`G-ENGINE-OPTION-HAND-LINK-COST-TIMING`), landed for the
sibling driver `BT25-093#effect#4` while this clause's verdict was still being written.
That commit moves the from-hand Plug-In **Option** link onto the §10-1-3 order: the host
question (`install_option_hand_link_host_selection`, a clone-safe
`ResumeFrame::OptionHandLinkHostSelection` data frame) is asked BEFORE anything is paid,
and `finish_option_hand_link` re-enters `play_option_core` with the host pinned. The
prompt is byte-identical to `install_link_host_selection`'s, so the wire meaning of the
host pick did not change — only its position in the procedure — which is why this
scenario needed no re-authoring.

Re-diffed against the SAME preserved sidecar, zero Unity time:

| clause | sidecar | result | verdict |
|---|---|---|---|
| `BT24-091#effect#4` | `20260921T042039Z_c3e0a78e` | **CLEAN** — compared 13 of 14 ours / 14 dcgo (1 sim-only row + 1 DCGO intermediate row not comparable) | **confirmed** |

The clause text is unchanged (sha `9d4716573dac469c1419dcbf9269cd84ee848a841a854e159fbc7d67eb6af6fa`,
byte-identical to the one hashed into the `diverged` verdict), so this is a real
re-measure of the same clause and not a drift-driven reset. Both the earlier
`unreachable` call and the `diverged` call are now superseded; neither the scenario nor
the YAML spec was touched.

Checked first, and ruled out: this was **not** the `field.N` centre-out slot-addressing
artefact that accounted for most of job 1's apparent divergences. The only two-Digimon
reference on this line is the host `select:`, which rides the wire as the target's card
identity (`BT25-024`), not as a slot index — see SLOT HYGIENE in the scenario header.
The divergence was a real procedure-order bug on the shared path.

Trackers updated: `docs/RUST_ENGINE_GAPS.md`
(`G-ENGINE-OPTION-HAND-LINK-COST-TIMING`, already RESOLVED — its Oracle bullet now
records all three drivers CLEAN) and `qa/resolved-gaps.md` (its Follow-up is now DONE).

**6 clauses: 6 confirmed, 0 diverged, 0 unreachable, 0 unavailable, 0 unmeasured.**
