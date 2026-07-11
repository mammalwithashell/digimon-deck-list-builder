# EX12 Shambala and Virus Busters Rust/DSL Scoping

Status: manual replacement for `wf_6f7700f2-6c5` on 2026-07-08.
The Claude workflow metadata was found, but its audit agents failed before
returning verdicts, so this document records the local Codex audit used by
`implement-ex12-shambala-virus-busters`.

## Source Checks

- Local card text came from `data/cards.json` / `code/digimon-engine/cards/ex12/*.json`.
- The initial audit had no local EX12 image assets. Later local passes used
  `digimon-card-lookup` to download and inspect sampled card scans for
  scan-sensitive cards including EX12-017, EX12-018, EX12-032, EX12-035,
  EX12-036, EX12-048, and EX12-065.
- Official rules source: Digimon Card Game Comprehensive Rules Manual
  `general_rule.pdf?20260619=`, sections 16-44 and 16-45.
- No YAML implementations were present under `code/digimon-engine/cards/ex12/`
  at audit time.

## Implementation Update

Local Codex pass on 2026-07-08 added YAML and DebugRunner behavioral coverage
for all 33 Shambala-slice cards in the OpenSpec scope. The focused additions at
the end of the pass were EX12-047 and EX12-074:

- EX12-047 uses a new `returned_card_color_count` formula selector over the
  current effect's returned-to-deck result log for the per-color DP reduction.
- EX12-074 uses a face-up-security `on_ally_attack` trigger for the attacking
  Shambala Digimon's hand digivolve, plus the standard Option lifecycle for its
  `[Main]` self-placement.

Verification:

- `cargo test -p digimon-engine --test cards_behavioral ex12_047 -- --test-threads=1`
  -> 3 passed.
- `cargo test -p digimon-engine --test cards_behavioral ex12_074 -- --test-threads=1`
  -> 3 passed.
- `cargo test -p digimon-engine --test dsl returned_card_color_count -- --test-threads=1`
  -> 2 passed.
- `cargo test -p digimon-engine --test cards_behavioral ex12_0 -- --test-threads=1`
  -> 78 passed.

The implementation wave task boxes remain open because the current pass did not
perform the planned external review/merge/commit gates. The
`DATA-EX12-PLACEHOLDER-TEXT` gap also remains applicable until the malformed
local EX12 inherited-text rows are corrected or scan-verified.

Local Codex Virus Busters passes on 2026-07-08 added YAML and DebugRunner
behavioral coverage for EX12-010, EX12-016, EX12-017, EX12-021, EX12-024,
EX12-032, EX12-035, and EX12-037. EX12-017's local JSON inherited text is still
malformed, but the printed card image was used to author the inherited Decode
clause. The follow-up pass scan-checked EX12-032 and EX12-035, added a reusable
same-level source-pair predicate for EX12-032, added EX12-035's printed Assembly
route, and used the new `repeat_effect_choice` vocabulary for EX12-037.

Verification:

- `cargo test -p digimon-engine --test cards_behavioral ex12_010 -- --test-threads=1`
  -> 3 passed.
- `cargo test -p digimon-engine --test cards_behavioral ex12_016 -- --test-threads=1`
  -> 3 passed.
- `cargo test -p digimon-engine --test cards_behavioral ex12_017 -- --test-threads=1`
  -> 3 passed.
- `cargo test -p digimon-engine --test cards_behavioral ex12_021 -- --test-threads=1`
  -> 4 passed.
- `cargo test -p digimon-engine --test cards_behavioral ex12_01 -- --test-threads=1`
  -> 26 passed.
- `cargo test -p digimon-engine --test cards_behavioral ex12_03 -- --test-threads=1`
  -> 16 passed.
- `cargo test -p digimon-engine --test cards_behavioral ex12_0 -- --test-threads=1`
  -> 115 passed.
- `cargo test -p digimon-engine --test dsl -- --test-threads=1`
  -> 914 passed.

## Resolved Keyword Questions

- `Engage`: section 16-44 says this is an optional trigger at End of Your Turn
  that lets the Digimon attack. The rules do not add Vortex's "opponent's
  Digimon only" target restriction or Vortex's played-this-turn exception, so
  implementation should use the normal attack legality path, including normal
  summoning-sickness checks.
- `Guard`: section 16-45 says this is an optional immediate-type replacement.
  When any of a player's other Digimon would leave the battle area by an
  opponent's effect, deleting the Guard carrier prevents that other Digimon from
  leaving.
- `Kotenken Token`: official EX12 card-list text and local JSON agree on
  `Digimon/Black/9000 DP/<Blocker>`. This was not scan-verified because the
  local EX12 image assets were unavailable.

## Current Substrate Notes

- `Guard` and `Engage` are now present in `Keyword`, the printed-keyword parser,
  `modifier_map::lookup_keyword`, and the DSL keyword allowlist.
- `Counter`, `Delay`, `Use Req.`, effect-initiated DNA digivolution,
  `may_attack_now`, attack redirection, force-attack, source trashing,
  source-to-hand/deck movement, top/bottom source placement, and protection
  against opponent Digimon/Option/Tamer effects already have engine/DSL
  substrate.
- The older protect-other-by-self-delete replacement gap is closed in the
  current DSL. `cost: { delete_self: true }` plus a leave-cancel outcome is
  available and should be reused for `Guard`.

## Consolidated Gaps

### G-KEYWORD-GUARD

Resolved 2026-07-08. `Guard` now has native keyword parse/lowering/runtime
support. It is not just a card-local clause because printed Guard and
aura-granted Guard must behave the same way. The runtime replacement exposes an
optional action when another own Digimon would leave the battle area by an
opponent's effect, deletes the Guard carrier as the cost, then cancels the other
Digimon's leave.

Consumers in this slice: EX12-056, EX12-057 Paishu Token. Future/adjacent
consumer noted by the OpenSpec: EX12-072 security grant.

### G-KEYWORD-ENGAGE

Resolved 2026-07-08. `Engage` now has native keyword parse/lowering/runtime
support and enqueues an optional End of Your Turn attack for carriers with
printed or granted Engage. Unlike Vortex, the rules text does not restrict the
target to an opponent's Digimon and does not bypass played-this-turn
restrictions.

Consumers in this slice: EX12-019. Future/adjacent consumer noted by the
OpenSpec: EX12-060.

### G-DSL-PLAY-OR-USE-FROM-SOURCES

Resolved 2026-07-08. EX12-077 now uses the source-origin
`play_or_use_from_sources` verb paired with `select_own_sources`, exposing the
mixed source-card selection and then routing Digimon/Tamer/Option/DUAL cards by
kind and face without hidden auto-selection.

Consumer in this slice: EX12-077 Proximamon.

### G-DSL-REPEAT-EFFECT-CHOICE

Resolved 2026-07-08. EX12-037 now uses `repeat_effect_choice` to expose one
modal effect choice for each full group of 5 cards in its digivolution cards,
with the repeat count snapshotted at activation time and nested selections
resuming before the next modal choice.

Consumer in this slice: EX12-037 Omnimon.

### G-DSL-SELF-SAME-LEVEL-SOURCE-PAIRS

Resolved 2026-07-08. EX12-032 now uses `self_same_level_source_pairs_gte` to
gate its [When Attacking] trash digivolve prompt on having at least two
same-level source cards.

Consumer in this slice: EX12-032 WereGarurumon.

### DATA-EX12-SCAN-ASSETS-MISSING

The local scan lookup could not verify EX12 card faces. Authoring can proceed
from JSON and official rules, but final no-approximations review should re-run
card lookup after assets are available.

### DATA-EX12-PLACEHOLDER-TEXT

Several local JSON rows contain placeholder or malformed text:
`EX12-014` has a malformed Decode reminder, and `EX12-017`, `EX12-019`,
`EX12-034`, `EX12-037`, `EX12-047`, `EX12-057`, `EX12-065`, `EX12-076`, and
`EX12-077` contain inherited-text placeholders such as `|applinkdp =`.

## Audit Index

Verdicts are scoped to Rust/DSL authorability from current local text. `SUPPORTED`
means the printed behavior appears expressible with existing substrate once YAML
is authored. `PARTIAL` means the card is blocked only by a keyword/data issue or
requires scan re-audit. `BLOCKED` means a reusable missing primitive is known.

| Batch | Card | Verdict | Gap IDs | Notes |
| --- | --- | --- | --- | --- |
| Virus Busters | EX12-001 Nyaromon | SUPPORTED | - | EOT DNA and may-attack shape is already present. |
| Shambala | EX12-002 Mococomon | SUPPORTED | - | Ally-played trigger and effect-initiated digivolve are present. |
| Shambala | EX12-004 Onibimon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Execute exists, but inherited text is truncated locally. |
| Virus Busters | EX12-005 Agumon | SUPPORTED | - | Hand-trash cost, draw, and inherited DP are standard. |
| Shambala | EX12-006 Kakamon | SUPPORTED | - | Hand-trash cost, draw, and memory gain are standard. |
| Virus Busters | EX12-007 Gammamon | SUPPORTED | - | Top reveal/add pattern is standard. |
| Shambala | EX12-009 Wankomon | SUPPORTED | - | Reveal/add two-filter search is standard. |
| Virus Busters | EX12-010 Greymon | SUPPORTED | - | Raid and trash recursion are present; YAML and behavioral tests green. |
| Shambala | EX12-011 Seasarmon | SUPPORTED | - | Raid and small-DP delete are present. |
| Shambala | EX12-012 Apemon | SUPPORTED | - | Optional hand-trash cost and draw are standard. |
| Virus Busters | EX12-013 BetelGammamon | SUPPORTED | - | `play_or_use_from_hand` covers the Main action. |
| Virus Busters | EX12-014 Canoweissmon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Decode behavior is supported, but local inherited text is malformed. |
| Shambala | EX12-015 Gokuumon | SUPPORTED | - | Alliance grant plus immediate attack is present. |
| Virus Busters | EX12-016 MetalGreymon | SUPPORTED | - | Decode and force-attack modifier are present; YAML and behavioral tests green. |
| Virus Busters | EX12-017 WarGreymon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Counter DNA, redirect, and scan-verified inherited Decode are implemented/tested; local JSON inherited text remains placeholder. |
| Virus Busters | EX12-018 Siriusmon / Planet Punch | SUPPORTED | - | Top/bottom source placement, highest-DP delete, and may-attack are present. |
| Shambala | EX12-019 Nezhamon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Engage is implemented; local inherited text remains placeholder. |
| Shambala | EX12-020 Gasamon | SUPPORTED | - | Cost reduction and hand-size draw gate are present. |
| Virus Busters | EX12-021 Gabumon | SUPPORTED | - | Start Main hand-trash cost and draw/memory are standard; YAML and behavioral tests green. |
| Shambala | EX12-022 Kamemon | SUPPORTED | - | Reveal/add and inherited hand-size draw are standard. |
| Virus Busters | EX12-024 Garurumon | SUPPORTED | - | Jamming, return-to-hand, and inherited draw/trash are present. |
| Shambala | EX12-025 Gawappamon | SUPPORTED | - | Blocker and level-lte bounce are present. |
| Shambala | EX12-026 Shellmon | SUPPORTED | - | Bottom-source trash and cannot-attack/block modifier are present. |
| Shambala | EX12-029 Sagomon | SUPPORTED | - | Suspend lock plus Alliance/may-attack is present. |
| Shambala | EX12-031 MarineBullmon | SUPPORTED | - | Decode, bottom-source placement cost, and bounce are present. |
| Virus Busters | EX12-032 WereGarurumon | SUPPORTED | - | Same-level source-pair gate and trash digivolve are present and tested. |
| Shambala | EX12-034 Erlangmon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT, DATA-EX12-SCAN-ASSETS-MISSING | Token stats are resolved from official text/JSON, not local scan. |
| Virus Busters | EX12-035 MetalGarurumon | SUPPORTED | - | Printed Assembly route, source trash, source-count bottom-deck, and suspend-lock observer are present. |
| Shambala | EX12-036 Ryugumon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Barrier/Evade/Decode and opponent effect locks are present. |
| Virus Busters | EX12-037 Omnimon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Multi-mode every-5-sources choice is implemented via `repeat_effect_choice`; local inherited text is placeholder. |
| Shambala | EX12-039 Takinmon | SUPPORTED | - | Cost reduction and Barrier are present. |
| Virus Busters | EX12-040 Salamon | SUPPORTED | - | Cost reduction and Barrier are present. |
| Virus Busters | EX12-042 Gatomon | SUPPORTED | - | Security-to-hand plus recovery is present. |
| Shambala | EX12-043 Hakubamon | SUPPORTED | - | `play_or_use_from_hand` covers the Main action. |
| Virus Busters | EX12-044 Angewomon | SUPPORTED | - | Same-level source gate and hand digivolve are present. |
| Shambala | EX12-045 Sanzomon | SUPPORTED | - | Security self-add, recovery gate, and security-removed observer are present. |
| Shambala | EX12-046 Shishimamon | SUPPORTED | - | Security Attack minus, DP minus, and security-removed digivolve are present. |
| Shambala | EX12-047 Amaterasumon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Ascension and color-count DP scaling are present; local inherited text is placeholder. |
| Shambala | EX12-048 SeitenGokuumon | SUPPORTED | - | Execute-like may attack, DP minus scaling, and source play on leave are present. |
| Shambala | EX12-056 Cho-Hakkaimon | SUPPORTED | - | Guard and the other clauses are present. |
| Shambala | EX12-057 Takutoumon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Paishu Token carries Guard; local inherited text is placeholder. |
| Shambala | EX12-061 Hanimon | SUPPORTED | - | Hand-trash cost and draw/trash inherited are standard. |
| Shambala | EX12-062 Kokeshimon | SUPPORTED | - | Delete-own cost and level-lte delete are present. |
| Shambala | EX12-063 Karakurumon | SUPPORTED | - | Suspend, unsuspend lock, and trash play are present. |
| Shambala | EX12-065 Kaguyamon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Fortitude and aura keyword grants are present; local inherited text is placeholder. |
| Virus Busters | EX12-066 Hiro Amanokawa | SUPPORTED | - | Tamer memory floor and suspend-for-digivolve/use-option mode choice are present. |
| Virus Busters | EX12-069 Virus Busters | SUPPORTED | - | Security attack-triggered play and self-security placement are present. |
| Shambala | EX12-070 Sanmyojin Arrival | SUPPORTED | - | Delay self-placement and leave-triggered play are present. |
| Shambala | EX12-071 Saneiketsu Invitation | SUPPORTED | - | Delay and effect-initiated digivolve are present. |
| Virus Busters | EX12-073 Giant Meat | SUPPORTED | - | Reveal/add, place-in-battle, and Delay memory are present. |
| Shambala | EX12-074 Genshi Continent and Ashino Island | SUPPORTED | - | Security-triggered digivolve and self-security placement are present. |
| Shambala | EX12-075 Kunlun's Imperial Decree | SUPPORTED | - | Reveal/add, place-in-battle, Delay memory, and security placement are present. |
| Shambala | EX12-076 Susanoomon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Source-color scaling and security placement/trash/recovery are present. |
| Virus Busters | EX12-077 Proximamon | PARTIAL | DATA-EX12-PLACEHOLDER-TEXT | Source-origin play-or-use choice is implemented and tested; local inherited text is placeholder. |
