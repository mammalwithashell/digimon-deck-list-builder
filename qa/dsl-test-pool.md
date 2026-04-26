# DSL Card Test Pool

Purpose: keep a small, real-card DSL pool that proves the loader, compiler,
runtime lowering, card tests, mechanic tests, and headless runner can all use
the same YAML pack before scaling to full-set coverage.

## First Vertical Slice

The first implemented slice is the Nokia / Greymon / Omnimon lane:

- `BT22-084` Nokia Shiramine: tamer memory floor, free Agumon/Gabumon play,
  persistent aura, security play.
- `BT17-007` Agumon: recursion setup and inherited DNA registration surface.
- `BT17-015` WarGreymon: cost reduction, branch choice, delete target,
  effect-initiated digivolve branch, inherited security pressure.
- `AD1-025` Omnimon: DNA path, Raid, Blocker, Partition marker, raw-rust body.
- `BT5-093` Tai Kamiya & Matt Ishida: tamer memory and Omnimon keyword aura.

This slice now has real YAML fixtures, per-card behavioral tests, a mechanic
test for the Nokia aura, and a headless matchup smoke using embedded DSL cards.

## Phase 3 Runtime Coverage

The Phase 3 DSL infra suite now covers the runtime features that unblock the
next pool-card migrations:

- Zone-count formulas: `card_count_in_zone` with `zone` and `of` payloads.
- Aggregate formulas scoped to controller, opponent, active player, or any
  player.
- Runtime `raw_rust` formula dispatch through an engine callback registry.
- Event trigger context for `event_target`, `event_card`, and related
  predicates.
- Next-turn scheduled effects and scheduled drains that resume after
  selections.
- `OnDnaDigivolve` triggers for both effect-initiated and user-action DNA.

Good next card candidates from the pool are `AD1-025`, `BT18-019`, and
`EX6-072` for DNA; `BT18-102` for formula-driven DP/material checks; and
`BT7-107` / `BT15-003` for scheduled or selection-heavy negative paths.

## Pool

| Card | Role | Primary coverage |
| --- | --- | --- |
| `BT9-092` Cool Boy | Generic X Antibody support | Search, reveal, multiple add-to-hand choices, security play |
| `BT7-107` Calling From the Darkness | Purple option staple | Delete own Digimon, trash-to-hand recursion, optional repeated choices |
| `BT11-042` Angewomon | Angel engine | Inherited aura, keyword grant, raw-rust handoff for security search/recovery |
| `BT13-007` King Drasil_7D6 | Royal Knights | Breeding flood gate, cost reduction, start-of-main source movement |
| `BT13-060` Rosemon: Burst Mode | Burst example | Burst digivolve, extra cost, suspend, CannotUnsuspend modifier |
| `BT14-009` Gotsumon | Flood gate | CannotPlayDigimonByEffect player restriction surface |
| `BT15-003` Nyaromon | Digi-Egg OPT | Inherited optional OPT, branch choice, security-as-cost |
| `BT17-007` Agumon | Slice rookie | Start-of-main recursion, inherited alt-path registration |
| `BT17-015` WarGreymon | Slice boss | Cost reduction, OnPlay/WhenDigivolving branch, deletion, inherited pressure |
| `BT18-019` Millenniummon | Millennium | DNA path, deletion, count-capped multi-select, trash recursion |
| `BT18-102` Susanoomon | Hybrid | Assembly, ACE overflow, formula DP target, material-to-security |
| `BT20-083` Omekamon | Liberator / RK | Blocker, optional free digivolve, material play, source placement |
| `BT22-084` Nokia Shiramine | Slice tamer | Memory floor, free play, aura, security play |
| `BT24-016` Lamiamon | Liberator | Activated digivolve, as-selecting-player, security swap, inherited free play |
| `AD1-025` Omnimon | Slice mega | DNA, Raid, Blocker, Partition, raw-rust mass removal handoff |
| `BT5-093` Tai Kamiya & Matt Ishida | Omnimon tamer | Start-of-turn memory, SecurityAttackPlus aura, security play |
| `BT10-111` Shoutmon (King Version) | Xros Heart | DigiXros, trash recursion, Material Save, inherited aura |
| `BT12-112` Shoutmon X7: Superior Mode | Xros Heart / Blue Flare | Wide DigiXros, return-to-deck, security-effect flood gate |
| `EX6-072` Mega Digimon Assembly! | DNA option | Color bypass, DNA raw-rust handoff, security trash-to-hand |
| `EX11-012` Medusamon | Liberator | Rush, Progress, deletion, optional follow-up, token play |
| `EX11-027` Maquinamon | ExMaquinamon | Search, link raw-rust handoff, alternate digivolve path |
| `ST2-13` Hammer Spark | Baseline option | Main/security memory gain |

## Coverage Map

- Search/reveal: `BT9-092`, `EX11-027`, `BT17-007`.
- Tamers/start timings: `BT22-084`, `BT5-093`, `BT9-092`.
- DigiXros/assembly: `BT10-111`, `BT12-112`, `BT18-102`.
- Modifiers/flood gates: `BT13-007`, `BT13-060`, `BT14-009`, `BT22-084`,
  `BT5-093`.
- Branch choices/optionality: `BT15-003`, `BT17-015`, `EX11-012`.
- Replacements/leave-field markers: `AD1-025`, `BT20-083`.
- DNA: `AD1-025`, `BT18-019`, `EX6-072`.
- Keywords: Raid, Blocker, Partition, Material Save, Rush, Progress,
  Security Attack +N, ACE overflow.
