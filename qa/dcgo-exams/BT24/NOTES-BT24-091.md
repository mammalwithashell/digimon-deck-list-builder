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
