# Archetype DSL Implementation: BG Imperial

Date: 2026-05-22

Assessment source: `data/deck_library.json` archetype `BG Imperial`, using the
two DigimonMeta 1st-place lists dated 2026-01-25 and 2026-02-14.

## Verdict

`implemented`

The deck-library pool contains 25 unique card IDs. All 25 have production YAML
under `code/digimon-engine/cards/` and focused Rust behavioral coverage. The
validated-card ledger owns 24 of those cards under `BG Imperial`; `BT17-077`
appears in the deck-library pool but is canonically tracked under `Royal
Knights` because it is a cross-archetype Imperialdramon: Paladin Mode card.

No BG Imperial pool YAML file contains a live, non-comment `raw_rust` clause,
step, formula, or helper reference.

## Reconciliation Notes

- `BT21-037` was the only stale ledger `PARTIAL` entry in the current pool. The
  old `G-DECLARATIVE-KEYWORD` blocker was stale, but the card was missing its
  printed `[Digivolve] [Veemon]: Cost 2` alternate path. That path is now
  authored in YAML and covered by `bt21_037_has_veemon_cost_2_alt_path`.
- `BT17-077` is counted as covered for the BG Imperial deck-library pool while
  remaining ledger-owned by `Royal Knights`. Its Paladin Mode trash-choice and
  returned-card rider are covered by the focused `bt17_077` test filter.
- `BT13-040` still has an adjacent raw-Rust helper,
  `bt13_040_may_play_veemon_from_hand_or_source`, but it is not in the BG
  Imperial deck-library pool. It should be handled as a separate Magnamon
  Armors/Royal Knights follow-up because existing union-zone free-play lowering
  supports hand/trash, not hand/material-source unions.

## Pool

| Card ID | Name | Ledger owner | Verdict | Notes |
|---|---|---|---|---|
| `BT12-002` | DemiVeemon | BG Imperial | IMPLEMENTED | Inherited When Attacking conditional draw |
| `BT12-021` | Veemon | BG Imperial | IMPLEMENTED | Search plus inherited end-of-turn DNA registration |
| `BT12-022` | ExVeemon | BG Imperial | IMPLEMENTED | DNA before-cost memory and inherited conditional Jamming |
| `BT12-028` | Paildramon | BG Imperial | IMPLEMENTED | DNA path, source trash, DNA-gated attack lock, inherited memory |
| `BT12-031` | Imperialdramon: Fighter Mode | BG Imperial | IMPLEMENTED | Source-return alt cost plus source-color aura |
| `BT12-047` | Wormmon | BG Imperial | IMPLEMENTED | Search plus inherited end-of-turn DNA registration |
| `BT12-050` | Stingmon | BG Imperial | IMPLEMENTED | DNA before-cost memory and inherited conditional Piercing |
| `BT16-025` | Paildramon | BG Imperial | IMPLEMENTED | Partition, source-count suspend, attack suspend/unsuspend branch |
| `BT16-027` | Imperialdramon: Fighter Mode | BG Imperial | IMPLEMENTED | Blast/ACE and source-count return |
| `BT16-028` | Imperialdramon: Dragon Mode | BG Imperial | IMPLEMENTED | Unsuspend lock and effect-initiated free digivolve |
| `BT16-040` | Wormmon | BG Imperial | IMPLEMENTED | Reduced effect digivolve chain from trash |
| `BT16-085` | Davis Motomiya & Ken Ichijoji | BG Imperial | IMPLEMENTED | Free play, delayed return, color gate, DNA source trash |
| `BT17-077` | Imperialdramon: Paladin Mode | Royal Knights | IMPLEMENTED | Cross-owned pool card; trash-choice rider covered |
| `BT17-097` | Return to the Primogenitor | BG Imperial | IMPLEMENTED | Free digivolve, Delay replacement, hand/trash security play |
| `BT20-020` | Imperialdramon: Fighter Mode | BG Imperial | IMPLEMENTED | Raid/Piercing, source-DP delete gate |
| `BT21-037` | Lighdramon | BG Imperial | IMPLEMENTED | Piercing, Armor Purge, Veemon cost-2 path, suspend/+DP |
| `BT3-002` | DemiVeemon | BG Imperial | IMPLEMENTED | Inherited draw gated by carrier Jamming |
| `BT3-093` | Davis Motomiya | BG Imperial | IMPLEMENTED | Start-turn memory, reveal-add, security free-play |
| `BT3-103` | Hidden Potential Discovered! | BG Imperial | IMPLEMENTED | One-shot future digivolve reducer with suspend cost |
| `EX1-014` | ExVeemon | BG Imperial | IMPLEMENTED | Jamming plus inherited carrier-only Jamming aura |
| `LM-030` | Green Scramble | BG Imperial | IMPLEMENTED | Main, Delay, security optional tail |
| `P-117` | Veemon | BG Imperial | IMPLEMENTED | Free-trait cost reduction and inherited two-color draw |
| `ST9-05` | Paildramon | BG Imperial | IMPLEMENTED | DNA return-bottom and OPT attack unsuspend |
| `ST9-06` | Imperialdramon Dragon Mode | BG Imperial | IMPLEMENTED | Source selection and free source play |
| `ST9-09` | Stingmon | BG Imperial | IMPLEMENTED | Play-cost reduction and inherited draw |

## Verification

- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt21_037 -- --nocapture`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt17_077 -- --nocapture`
