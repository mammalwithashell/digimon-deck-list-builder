# Appmon run briefing — shared implementer context (2026-06-12)

Read this ENTIRE file before writing any test or YAML. It supplements your per-card prompt.
Also MANDATORY reading before writing tests: `docs/RUST_DSL_TEST_API.md` §5 (per-card test pattern), §8 (selection patterns), §11 (anti-patterns), §14 (Tracks A–K patterns). When you need a DSL verb's parameters, Read `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`; for `EffectContext` signatures, Read `docs/RUST_ENGINE_API.md`.

## 1. CRITICAL — the Appmon link box (API text drops it)

The per-card JSON (`effect_description_eng` etc.) was ingested from an API that **silently drops the
Appmon link box** printed at the bottom of every Appmon card. Your prompt contains the CORRECTED
printed text transcribed from the card image — it is authoritative. The link box has up to three parts,
each with an established DSL modeling (all verified against the engine, 2026-06-12):

1. **Link requirement** — "Link [Appmon] trait: Cost N":
   ```yaml
   - kind: link_condition
     cost: N
     filter: { trait_has: Appmon }
   ```
   (BT25-007/036/052/056/061/070/072 precedent.)

2. **Link DP bonus** — the printed "+N DP" icon in the link box. While this card is linked to a host,
   the host gets +N DP. Model as a linked-scope self-aura:
   ```yaml
   - scope: linked
     kind: aura
     target: {}
     dp_modifier: N
     summary: "[Link] +N DP to the linked Digimon"
   ```
   VERIFIED end-to-end: `Game::static_dp_aura_bonus` (code/digimon-engine/src/game/queries.rs) scans
   `permanent.linked_cards` for `.linked()` declarative dp auras (comment "DigiLink Shape-B
   (G-LINK-INHERITED-ESS)"). NOTE: the 9 already-shipped BT25 link cards OMIT this clause — do NOT
   copy that omission; it is a known drift being filed separately. Write a behavioral test: link the
   card to a host, assert `runner.effective_dp(host)` rose by N; unlink/delete → bonus gone (or at
   minimum assert presence while linked).

3. **Linked effect** — extra text in the link box:
   - "[When Linking] <effect>" → triggered clause with linked scope:
     ```yaml
     - scope: linked
       when: when_linked
       process: [...]
     ```
     Fires when THIS CARD gets linked to a host (BT25-007's "[When Linking] Delete 1 of your
     opponent's Digimon with 3000 DP or less" is the canonical exemplar).
   - A keyword printed in the link box (e.g. Raid on BT21-009) → granted to the HOST while linked:
     ```yaml
     - scope: linked
       kind: grant_keyword
       keyword: Raid
       summary: "[Link] <Raid> (host gains Raid while this card is linked)"
     ```
     (BT25-101's `scope: linked` Reboot grant is the precedent.)

   DCGO encodes these as the `SetIsLinkedEffect(true)` / "ESS - When Linked" blocks — they ARE printed
   text (the link box), despite earlier scout briefs claiming otherwise.

## 2. YAML traits must carry the FULL merged trait line

Production `CardData::load_from_str` merges `form_eng + attribute_eng + type_eng` into `traits`.
DebugRunner DSL cards get traits ONLY from your YAML `traits:` list (`attribute:` is a separate
compiled field consumed by `attribute_is` predicates — it is NOT folded into traits for DSL-loaded
cards). All Appmon machinery keys off `trait_has` (Appmon link conditions, "Stnd."/"Sup." digivolve
gates, Social/Navi/Tool link filters), so:

- Put the complete trait line from the CARD IMAGE into `traits:`, including the form segment
  ("Stnd.", "Sup.", "God"), "Appmon", the app-attribute (e.g. "Social"), and the type traits
  (e.g. "Search", "Hero").
  Example — BT21-009 image trait line `Stnd./Appmon | Social | Search/Hero`:
  ```yaml
  traits: ["Stnd.", Appmon, Social, Search, Hero]
  attribute: Social
  ```
- Type strings like "Search (App Name)" in the JSON mean app-name trait "Search" — use the bare name.

## 3. Digivolve boxes on Appmon Digimon

These cards print TWO digivolve requirements (left-edge circles): the standard color/level one
(carried in JSON `evo_costs`, becomes engine evo_costs — declare it in YAML the same way BT25 cards
do, i.e. via the standard `evo:` / cost fields if the schema supports it, or alt_paths) and a
form-gated one ("Stnd.: 3" / "Sup.: 4") modeled as an alt path:
```yaml
alt_paths:
  - kind: digivolve
    from: { level_eq: L, trait_has: "Stnd." }
    cost: C
```
(BT25-036 precedent. Read the DCGO C# `AddSelfDigivolutionRequirementStaticEffect` call to confirm
the level gate L for your card.) App Fusion boxes are:
```yaml
  - kind: app_fusion
    materials:
      - filter: { name_in: [NameA, NameB, ...] }
    cost: 0
```

## 4. Positive rules (the "C" half of the hybrid checklist — every item is review-enforced)

1. TDD ordering is strict: write the test file FIRST, run it, show the failing output, then author YAML.
2. File header docstring mandatory: verbatim card text (the CORRECTED text from your prompt), DCGO ref
   path, pattern row tags from the scout brief.
3. One positive AND one negative test per condition — split, never combined.
4. Every clause gets ≥1 integrated test driven through `play` / `attack` / `end_turn` / link flow.
5. OPT clauses get an explicit lockout test (second activation gated; lockout clears after end_turn).
6. Cost-firing clauses get an event-log test via `events_since(checkpoint)`.
7. Use `dsl_card(id)`, never inline-paste production YAML.
8. Use `digimon_engine::action::space::*` constants, never hard-code action IDs.
9. No approximations: every player choice surfaces through `pending_selection`. No auto-picks.
10. No Python references (`code/engine_py_legacy/` is out of scope).
11. Before declaring BLOCKED, grep `docs/RUST_ENGINE_GAPS.md` and `qa/dsl-vocab-gaps.md` and read the
    relevant DSL-spec section; set `gap_kind` = engine | dsl | hybrid accurately.
12. No `place_on_field` shortcuts when testing OnPlay paths (post-play state only).
13. No `auto_resolve` through a multi-branch prompt when testing a specific branch.

## 5. Exemplars (closest shipping YAMLs for this archetype)

- `code/digimon-engine/cards/bt25/BT25-007.yaml` — Gatchmon: link_condition + alt digivolve trait gate
  + on_play reveal buckets + `scope: linked, when: when_linked` delete.
- `code/digimon-engine/cards/bt25/BT25-052.yaml` — when_card_linked_to_this OPT + count_lte tamer gate
  + select_hand name_is + play_from_hand_free.
- `code/digimon-engine/cards/bt25/BT25-056.yaml` / `BT25-072.yaml` — link_card_to_self from
  [hand, digivolution_sources], multi-timing, when_card_linked_to_this.
- `code/digimon-engine/cards/bt25/BT25-036.yaml` — app_fusion alt path + "Stnd." digivolve gate.
- `code/digimon-engine/cards/bt25/BT25-070.yaml` — when_card_linked_to_this + OPT + your_turn +
  select_opponent_permanent + delete. Its test `tests/cards_behavioral/bt25/bt25_070.rs` has the
  canonical link-driving recipe (`fire_main` helper).
- `code/digimon-engine/cards/bt25/BT25-101.yaml` — linked-scope aura (security_attack) + linked-scope
  grant_keyword (Reboot) + link_cards step.
- `code/digimon-engine/cards/bt25/BT25-018.yaml` — `dp_lte: { formula: { source_dp: {} } }` ceiling.

## 6. Running your card's tests

The test-discovery `mod.rs` lines for your card are already wired (an empty placeholder `.rs` exists —
overwrite it with your test file content):
```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- <card_id_lower>
```

## 7. Worktree guardrails (MANDATORY)

- You run in an isolated git worktree. FIRST ACTION: run `git rev-parse --show-toplevel` and verify the
  path contains `.claude/worktrees/` (or a worktree temp dir) — if it resolves to the base repo
  `digimon-deck-list-builder-1` root, ABORT and report.
- Write files ONLY via paths relative to your starting cwd.
- The DCGO C# path in your prompt is an absolute base-repo path: READ-ONLY. Never `cd` there, never
  treat it as project root, never write near it.
- Deliverables are EXACTLY two files: your card's YAML and your card's test `.rs`. Do NOT touch any
  `mod.rs`, `main.rs`, tracker JSON, or this briefing.
