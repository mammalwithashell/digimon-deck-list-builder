# Construct semantics: identity-state-predicates-and-protection

- **Family:** identity-state-predicates-and-protection
- **Date:** 2026-10-22
- **Worktree / HEAD:** `D:/wt/p3-constructs` @ f24f26dfa (read-only research)
- **Constructs (in order):**
  - `predicate:card_number_is`
  - `predicate:name_in`
  - `predicate:trait_contains`
  - `predicate:play_cost`
  - `predicate:source_count`
  - `predicate:self_color_count_gte`
  - `predicate:face_down_sources_under_tamers_gte`
  - `predicate:on_field`
  - `predicate:can_attack`
  - `predicate:can_change_orientation`
  - `verb:grant_narrow_opponent_effect_protection`
  - `verb:grant_stack_protection_from_opponent_effects`
  - `verb:suspend_and_cancel_all_matching_leaves`

Paths are relative to `code/digimon-dsl/src/` ("DSL") or `code/digimon-engine/src/` ("ENG") unless written in full.

## Read this first (applies to every section)

### 1. How the coverage tool credits a test

The rules are in the module docstring of `code/tools/lowering_coverage.py`:

- The `#[test]` fn must live in a counted directory: `tests/dsl`, `tests/rules_conformance`, `tests/keyword_phase_*`, `tests/timing_dispatch*`, `tests/cost_hooks`, `tests/replacements*` or `tests/effect_context*`.
- Its body, or a same-file helper/const it references by name, must contain a **string literal that parses as YAML and uses the construct key**. Examples: `can_attack:` and `play_cost:`.
  - `play_cost:` here means the *canonical* comparator key. The legacy `play_cost_lte:` / `play_cost_gte:` keys do NOT credit `predicate:play_cost`.
- The body must also hit an `ENGINE_RUN_MARKERS` pattern (`DebugRunner`, `Game`, `EffectContext`, `runner.game`, ...).
- `parse_*.rs` files never count as behavioural.
- Put the leaf directly under a `filter:` / `condition:` / `active_when:` key. That is the safest way to get credit.

### 2. The predicate subject decides which leaves are evaluated

Where a leaf sits determines its subject:

- **`Permanent(h)` → `eval_permanent_fields` (ENG `dsl_cards/predicate.rs:4077`).** This is the subject for the `filter:` of `select_own_permanent`, `select_opponent_permanent` and `any_permanent`, and for a `count_gte` over the battle area.
- **`Card` / `RevealedCard` → `eval_card_fields` (`predicate.rs:3189`).** This is the subject for the `filter:` of `select_hand` and `select_trash`, for `count_gte {zone:[hand]}`, for reveal buckets and for source filters.
- **`Permanent(source_permanent)`.** This is the subject for a triggered clause's `condition:` / `active_when:` (`lower_triggered.rs:452 predicate_subject_for_source`). It falls back to `None` after a deletion.
- **`Permanent(source)`.** This is the subject for a self-aura `active_when:` (`lower_aura.rs:683`).
- **`PredicateSubject::None`.** This is the subject for `if: { condition: ... }` inside `process:` (`step/control_flow.rs:51-58`).

On a `None` subject, `eval_no_subject_fields` (`predicate.rs:1529-1572`) behaves as follows:

- It returns false if a listed card-identity leaf is present. The listed leaves include `trait_contains`, `name_in`, `card_number_is`, `play_cost_*` and `self_color_count_gte`.
- The list omits `source_count`, `can_attack` and `can_change_orientation`, as well as `is_suspended` and `link_card_count`.
- Those omitted leaves are silently ignored, so they pass vacuously. The same happens on a `Card` subject.
- Always test them with a Permanent subject.

### 3. Harness idioms

Copy these from `tests/dsl/effect_battle.rs` and `tests/paid_prevention_cardinality.rs`.

- **Build the runner:**
  ```rust
  DebugRunner::builder()
      .from_dsl_yaml(Y)?
      .add_card(make_test_card(..))
      .hand(..).security(..).memory(..)
      .start()
  ```
- **Place cards:**
  - `place_on_field(p, id, Some(0))` gives a permanent that is not fresh.
  - `place_on_field(p, id, None)` gives a permanent played this turn.
  - Also `place_stack(p, &[bottom.., top])`, `push_source(host, id)` and `place_in_breeding`.
- **Fire a trigger:**
  ```rust
  runner.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
  runner.game.drain_effect_queue();
  ```
  The controller is the permanent's owner. An opponent fixture is therefore placed on player 1's field and fired the same way.
- **Read and answer a prompt:**
  - `runner.game.pending_selection` gives `is_optional`, `valid_action_ids` and `selecting_player`.
  - Answer it with `runner.execute_action(p, a)`.
  - For optional replacements use `REPLACEMENT_ACCEPT` or `PASS`.
- **Make a source face-down:** `runner.game.players[p].battle_area[i].card_sources[j].face_down = true`. Index 0 is the bottom card.
- **Install modifiers:**
  - `runner.game.modifiers.add(h, ModifierEntry::simple(ModifierType::X, v, Expiry::.., source_player))`
  - `runner.game.modifiers.grant_keyword(h, Keyword::Rush, Expiry::Permanent, 0)`
  - Player modifiers use `add_player_modifier`.

---

### `predicate:card_number_is`

- **Lowering site(s):**
  - Parse: DSL `predicate.rs:171` (`pub card_number_is: Option<String>`). Compiled form: `compiled.rs:416`, `compile.rs:1015`.
  - Do not confuse it with the face-up-security leaf. `FaceUpSecurityNamedPredicate` has a field of the same name (`predicate.rs:1520`). That field is evaluated at ENG `predicate.rs:340` and compares case-insensitively. It is a separate leaf, not this construct.
  - Runtime, Card subject: ENG `predicate.rs:3552-3556`:
    ```rust
    if let Some(ref cn) = pred.card_number_is {
        if data.card_id != *cn {
            return false;
        }
    }
    ```
  - Runtime, Permanent subject: delegated through `eval_permanent_fields` to `eval_card_fields(top card)` (`predicate.rs:4329-4340`). There is no overlay handling.
- **YAML shape:**
  - The bare leaf is `card_number_is: "BT24-030"`.
  - In production it almost always appears as a "this card" proxy inside an existential:
    ```yaml
    - any_permanent:
        of: you
        zone: [battle_area]
        card_number_is: "BT24-030"
        is_unsuspended: true
    ```
  - It also appears as a `flood_gate` target: `target: { card_number_is: "P-180" }`.
- **Semantics:**
  - The leaf is an exact, case-sensitive equality test of the printed `CardData.card_id` against the string.
  - On a permanent it reads the top card's printed id, with no synth overlay.
  - Alt arts and parallel prints all match. `cards.json` has no alt-art suffixes: all 4475 keys have the form `SET-NNN`, so these prints share one id.
  - A reprint under a different number, such as a `P-xxx` reprint, does **not** match.
  - The `(Rule) Card number: Also treated as [XX]` alias (§2-3-4-5) is not modelled anywhere, so it never matches. RB1-004, RB1-006 and RB1-007 carry this alias.
  - On a `None` subject (`if:`) the leaf is false.
  - The leaf surfaces no selection.
- **Rule basis:**
  - §2-12-1: cards with matching card names and card numbers are the same card.
  - §2-3-4-5-1: a (Rule) card-number alias is also treated as `[XX]`. Not modelled.
  - §1-4-1-2-2: the 4-copy limit is counted per card number.
  - Otherwise this is a card-text idiom meaning "this card".
- **Production users:**
  - **`BT24-030` Neptunemon** (decks 3-Jupitermon and 8-Jupitermon).
    - Printed: "[All Turns] When any of your Digimon with the [TS] trait or [Aqua] or [Sea Animal] in any of their traits would leave the battle area by your opponent's effects, by suspending this Digimon, they don't leave."
    - YAML (`cards/bt24/BT24-030.yaml:78`), inside the replacement's `active_when`:
      ```yaml
      any_permanent: { of: you, zone: [battle_area], card_number_is: "BT24-030", is_unsuspended: true }
      ```
  - **`BT24-102` Homeros** (TS decks).
    - Printed: "[End of Your Turn] By suspending this Tamer, you may activate 1 [On Play] or [When Digivolving] effect of 1 of your [Olympos XII] trait Digimon."
    - YAML (`cards/bt24/BT24-102.yaml:56`), followed by `suspend: { target: source }`:
      ```yaml
      condition: { any_permanent: { of: you, zone: [battle_area], card_number_is: "BT24-102", is_unsuspended: true } }
      ```
  - It is also used as a flood_gate target, e.g. in `P-180` and `BT25-098` with `target: { card_number_is: "P-180" }` (IgnoreColorRequirement).
- **Existing near-miss tests:**
  - `tests/dsl/parse_identity.rs` is a `parse_*` file. It deserializes `identity.name_aliases[].when.has_inherited.card_number_is` and asserts the string. It never runs the engine.
  - `src/dsl_cards/predicate.rs` and `src/dsl_cards/lower_replacement.rs` are production source only. The token appears in the eval and in `predicate_reads_replacement_subject`.
- **Proposed behavioural test (for the plan):**
  - Fixture:
    ```yaml
    card: TST-CNUM
    name: Card Number Probe
    kind: tamer
    color: [red]
    cost: 2
    effects:
      - when: on_play
        process:
          - select_hand:
              of: you
              bind_as: pick
              optional: true
              filter: { card_number_is: "TST-TARGET" }
              prompt: "Pick TST-TARGET"
      - when: on_play
        condition:
          any_permanent: { of: you, zone: [battle_area], card_number_is: "TST-TARGET" }
        process:
          - gain_memory: 1
    ```
  - Supporting cards:
    - `make_test_card("TST-TARGET","Agumon")`
    - `make_test_card("TST-OTHER","Agumon")`: same name, different number.
    - `make_test_card("TST-TARGETX","Agumon")`: a prefix collision.
  - Setup:
    - hand(0) = [TST-TARGET, TST-OTHER, TST-TARGETX, TST-TARGET].
    - Place TST-CNUM on P0's field.
    - For case B, also place TST-TARGET on P0's field.
  - Positive cases:
    - Enqueue OnPlay and drain. The `select_hand` prompt has exactly **2** `valid_action_ids` (the two TST-TARGET copies) and `is_optional == true`.
    - With TST-TARGET on P0's field, memory rises by 1. Assert the `runner.memory()` delta.
  - Negative cases:
    - TST-OTHER (same name, different number) is never offered.
    - TST-TARGETX (prefix) is never offered.
    - With no TST-TARGET on P0's field, or TST-TARGET only on P1's field, memory is unchanged.
    - With the filter written as `card_number_is: "tst-target"` (lower case), assert that 0 cards are offered and there is no prompt. This pins the current case-sensitive behaviour. The plan decides whether case-insensitivity is wanted.
  - Selection surface: the `optional: true` select must be a real `pending_selection`, with no auto-pick even when only 1 candidate exists. Check this with a single-copy hand variant.
- **Suspected engine/DSL defects (read-only):**
  1. **The self-proxy idiom over-matches duplicates.**
     - About 25 production sites use `any_permanent { card_number_is: <own id>, is_unsuspended: true }` to mean "THIS card is unsuspended".
     - With two copies on the field, the suspended carrier passes the gate because the *other* copy is unsuspended.
     - The cost step `suspend: {target: source}` then no-ops, while the rest of the body still runs. Homeros BT24-102, Lui Ohwada P-130 and Owen Dreadnought BT21-081 all pay nothing and still resolve.
     - This violates §15-7-2 and §15-7-4.
     - The correct leaf, `source_is_unsuspended: true`, already exists.
  2. The §2-3-4-5 card-number alias is not modelled. Impact is low.
  3. The case-sensitive compare is inconsistent with the face-up-security `card_number_is`, which uses `eq_ignore_ascii_case`. Cosmetic.

---

### `predicate:name_in`

- **Lowering site(s):**
  - Parse: DSL `predicate.rs:156` (`name_in: Option<Vec<String>>`). Compiled form: `compiled.rs:408`, `compile.rs:1008`.
  - Runtime, Card subject: ENG `predicate.rs:3512-3523`. It goes through `any_physical_card_name_matches` (`3178`), which wraps `any_effective_name_matches` (`3138`):
    ```rust
    if let Some(ref names) = pred.name_in {
        let overlay_name = overlay.and_then(|o| o.name.as_deref());
        if !any_physical_card_name_matches(&data.card_name, overlay_name,
            &data.also_treated_as, runtime_names,
            |name| names.iter().any(|n| n.as_str() == name)) { return false; }
    }
    ```
  - Runtime, Permanent subject: `predicate.rs:4182-4196`.
    - It matches against `synth_identity.card_names`: the printed name, `also_treated_as`, and any `ChangeBaseCardName` overlay. DNA-result extras also count.
    - If that match fails it **returns false**. It does not fall back to the printed name, because the overlay *replaces* names.
  - **App Fusion is a separate, non-predicate path.**
    - `app_fusion` alt-paths do NOT evaluate `name_in` through the predicate evaluator.
    - `dna_digivolve.rs:1521 app_fusion_condition_names` collects the list.
    - `card_name_equals` (`:1557`) compares the **printed name only**, case-insensitively.
- **YAML shape:**
  - As a filter: `name_in: ["Veemon", "Wormmon"]`.
  - Within a material: `materials: [{ filter: { name_in: [Onmon, Gatchmon] } }]`.
- **Semantics:**
  - The leaf is true when ANY effective name of the subject equals ANY listed string **exactly** (case-sensitive, whole name).
  - The effective names are:
    - the printed name
    - the reveal-overlay name (Card subject)
    - static `also_treated_as` aliases
    - runtime physical aliases (`CardSource.also_treated_as`)
    - on the field, the synthesized names
  - It is exact-name matching (§2-3-1-2). So the Language Standardization "in its name" exclusions are irrelevant here. Examples of those exclusions: BurningGreymon ≠ [Greymon] and Indramon ≠ [Dramon] (memory `reference_name_aliases_and_dcgo_name_exclusions`). `name_in: [Greymon]` never matches "BurningGreymon" anyway.
  - A `(Rule) Name: Also treated as having [X] in its name` alias (§2-3-4-4) is stored as a composite alias string, e.g. "Mamemon Thundermon". So it correctly does not make `name_in: [Mamemon]` match.
  - `data/archetype_aliases.json` holds **deck-archetype** names, e.g. "TS Jupitermon": ["TS Jupiter", "Aegiomon"]. This leaf does NOT consult that file and must not.
  - A `None` subject gives false.
  - The leaf surfaces no selection itself.
- **Rule basis:**
  - §2-3-1-2: bracketed names reference exactly that name.
  - §2-3-4-3-1: a (Rule) "also treated as [XX]" name counts.
  - §2-3-4-4-1: "also treated as having [XX] in its name" does NOT make the card named [XX].
  - §2-3-1-4: "ACE" is not part of the name. The data already strips it.
- **Production users:**
  - **`BT25-052` Logimon** (deck 2-Rebootmon). Every top-10 use of `name_in` is App Fusion.
    - Printed App Fusion: "[Onmon] & [Gatchmon]: Cost 0".
    - YAML (`cards/bt25/BT25-052.yaml:84`):
      ```yaml
      - kind: app_fusion
        materials: [ { filter: { name_in: [Onmon, Gatchmon] } } ]
        cost: 0
      ```
    - **This goes through `app_fusion_condition_names`, not the predicate evaluator.**
  - **`BT16-085` Davis Motomiya & Ken Ichijoji** (predicate path).
    - Printed: "[Start of Your Main Phase] You may play 1 [Veemon] or [Wormmon] from your hand without paying the cost."
    - YAML (`cards/bt16/BT16-085.yaml:141,155`):
      - the condition is `count_gte { filter: { of: you, zone: [hand], name_in: ["Veemon","Wormmon"] }, n: 1 }`
      - the pick is `select_hand { optional: true, filter: { name_in: ["Veemon","Wormmon"] } }`
- **Existing near-miss tests:** none. `src/dsl_cards/predicate.rs` and `src/dsl_cards/lower_replacement.rs` are source only, and no test references the leaf.
- **Proposed behavioural test (for the plan):**
  - Fixture:
    ```yaml
    card: TST-NAMEIN
    name: Name In Probe
    kind: tamer
    color: [blue]
    cost: 2
    effects:
      - when: on_play
        process:
          - select_hand:
              of: you
              bind_as: pick
              optional: true
              filter: { name_in: ["Veemon", "Wormmon"] }
              prompt: "Pick Veemon or Wormmon"
      - when: on_play
        condition:
          any_permanent: { of: you, zone: [battle_area], name_in: ["Veemon", "Wormmon"] }
        process:
          - gain_memory: 1
    ```
  - Hand cards:
    - "Veemon", "Wormmon", "ExVeemon", "DemiVeemon", and "veemon" (lower-case name).
    - A YAML card `TST-ALIAS` named "Flamedramon" with `also_treated_as: [Veemon]`. Check the authoring key in `docs/RUST_DSL_AGENT_GUIDE.md`.
  - Setup: hand(0) holds those cards. TST-NAMEIN is on P0's field.
  - Positive cases:
    - `select_hand` offers exactly 3 cards (Veemon, Wormmon, TST-ALIAS), with `is_optional == true`.
    - Place a "Veemon" on P0's field → memory +1.
    - Place a plain "Agumon" on P0's field with `ChangeBaseCardName { value: "Wormmon" }` → memory +1. The overlay counts.
  - Negative cases:
    - ExVeemon and DemiVeemon (substring matches) are not offered.
    - The lower-case "veemon" is not offered (case-sensitive).
    - An "Agumon" permanent with `ChangeBaseCardName` → "Gabumon" fails `name_in: [Agumon]`. The overlay replaces the printed name (`predicate.rs:4189-4196`).
    - With no matching permanent, memory is unchanged.
  - Selection surface: a real pending_selection, optional per the "you may" text.
  - App Fusion: give it its own test, because it is a different code path. See defect 1.
- **Suspected engine/DSL defects (read-only):**
  1. **App Fusion `name_in` ignores aliases and is case-insensitive.**
     - `card_name_equals` reads only `card.card_name`, the printed name.
     - A host or link card that is "also treated as [Onmon]" (§2-3-4-3) would not satisfy App Fusion, while the predicate path accepts it.
     - So behaviour differs depending on where `name_in` is written.
  2. **Overlay names are handled asymmetrically.** A Card subject treats a reveal-overlay name as an *alternative* (printed OR overlay). A Permanent subject treats it as a *replacement*. This is documented at `predicate.rs:3524-3535`, but the asymmetry is worth pinning in tests.

---

### `predicate:trait_contains`

- **Lowering site(s):**
  - Parse: DSL `predicate.rs:98`. Compiled form: `compiled.rs:388`, `compile.rs:1000`.
  - Runtime, Card subject: ENG `predicate.rs:3401-3409`:
    ```rust
    if let Some(ref t) = pred.trait_contains {
        let needle = t.to_lowercase();
        if !data.traits.iter().any(|x| x.to_lowercase().contains(&needle)) { return false; }
    }
    ```
  - Runtime, Permanent subject: the leaf is first checked against `synth_identity.traits` (`predicate.rs:4118-4124`).
    - On an overlay match the field is cleared from the delegated predicate (`4289-4291`).
    - Otherwise the printed check above runs on the top card.
- **YAML shape:** `trait_contains: Aqua` or `trait_contains: "Sea Animal"`. It is usually written inside `any_of` together with `trait_has`.
- **Semantics:**
  - The leaf is true if any trait string of the subject contains the needle as a case-insensitive substring.
  - `data.traits` is the whole trait line: form + attribute + type. It includes Rule-granted traits folded in by the DSL bridge (memory `project_dsl_trait_line_fold`).
  - Examples: `Aqua` matches Aquatic, Aquabeast and Ancient Aquabeast. `saur` matches Dinosaur, Plesiosaur and Ankylosaur.
  - A card with no traits gives false.
  - On the field, a `ChangeTraits`-granted trait also counts.
  - A `None` subject gives false.
  - No selection.
- **Rule basis:**
  - §2-3-2-4: "with [XX] in any of its traits" references traits that include the text.
  - Contrast §2-3-2-3, the exact "[XX] trait" form, which is `trait_has`.
  - §2-3-2-1: the trait line includes form and attribute.
  - §2-3-4-2: Rule-granted traits count.
- **Production users:**
  - **`EX12-031` MarineBullmon** (deck 7-Toho Braves).
    - Printed: "By placing 1 level 6 or lower card with [Aqua] or [Sea Animal] in any of its traits or the [TB] trait from your hand as this Digimon's bottom digivolution card, ..."
    - YAML (`cards/ex12/EX12-031.yaml:49`):
      ```yaml
      count_gte:
        filter:
          of: you
          zone: [hand]
          level_lte: 6
          any_of: [ { trait_contains: Aqua }, { trait_contains: "Sea Animal" }, { trait_has: TB } ]
        n: 1
      ```
    - The Assembly material filter at `:33` uses the same shape.
  - **`BT26-005` Pinamon** (deck 10-DATA SQUAD Ravemon).
    - Printed: "... you may play 1 play cost 5 or lower [Avian] or [DATA SQUAD] trait card from your trash ..."
    - YAML (`cards/bt26/BT26-005.yaml:50`):
      ```yaml
      select_trash:
        filter:
          all_of:
            - any_of: [ { trait_contains: "Avian" }, { trait_has: "DATA SQUAD" } ]
            - play_cost_lte: 5
      ```
- **Existing near-miss tests:** none. `src/dsl_cards/predicate.rs` is source only.
- **Proposed behavioural test (for the plan):**
  - Fixture:
    ```yaml
    card: TST-TRAITC
    name: Trait Contains Probe
    kind: tamer
    color: [green]
    cost: 2
    effects:
      - when: on_play
        process:
          - select_hand:
              of: you
              bind_as: pick
              optional: true
              filter: { trait_contains: "saur" }
              prompt: "Pick a card with [saur] in any trait"
      - when: on_play
        condition:
          any_permanent: { of: you, zone: [battle_area], kind: digimon, trait_contains: "AQUA" }
        process:
          - gain_memory: 1
    ```
  - Hand cards (DSL YAML, or `make_test_card` with `traits` set):
    - `traits: [Dinosaur]`
    - `traits: [Plesiosaur]`
    - `traits: [Dragonkin]`
    - `traits: [Saurian]`, with a capital S
    - one card with no traits
  - Positive cases:
    - The `select_hand` prompt offers 3 cards: Dinosaur, Plesiosaur and Saurian (the match is case-insensitive).
    - A P0 Digimon with `traits: [Aquabeast]` → memory +1.
    - A plain Digimon given `ChangeTraits { add: [Aquatic] }` → memory +1.
  - Negative cases:
    - Dragonkin and the no-trait card are not offered.
    - A `traits: [Sea Animal]` Digimon does not satisfy `AQUA`.
    - With no matching permanent, memory is unchanged.
  - Selection surface: an optional pending_selection, with no auto-pick.
  - Also add one contrast assertion, `trait_has: Dinosaur` versus `trait_contains: Dino`, so the difference between substring and exact matching is pinned.
- **Suspected engine/DSL defects (read-only):**
  1. **A removed printed trait still matches.**
     - If `ChangeTraits { replace: true }` removes a permanent's printed trait, the permanent still matches the leaf.
     - A miss against the synth identity falls through to the printed `data.traits`. The `name_*` leaves deliberately do not fall through this way.
     - The same issue applies to `trait_has`. Impact is low.
  2. **BT26-005 uses substring matching for an exact-trait text.**
     - The card prints the exact "[Avian] trait" form (§2-3-2-3), but it is authored as `trait_contains: "Avian"` after DCGO's `ContainsTraits`.
     - There is no observable difference today, because the only trait containing "avian" is "Avian".

---

### `predicate:play_cost`

- **Lowering site(s):**
  - Parse: DSL `predicate.rs:1115` (`play_cost: Option<MetricComparators>`). `MetricComparators` and `Comparator` are defined at `predicate.rs:1295-1340`.
  - Compile: `compile.rs:921-927` calls `resolve_metric_comparators` (`compile.rs:764-804`). That function lowers the key into the compiled `play_cost_eq`, `play_cost_lte` and `play_cost_gte` slots.
  - Runtime: ENG `predicate.rs:3557-3571`. `ref_play_cost` is computed at `3214-3217`:
    ```rust
    let ref_play_cost: i32 = match (&data.dual, option_ref) {
        (Some(d), true) => effective_use_cost(rctx, card).unwrap_or(i32::from(d.option.use_cost)),
        _ => i32::from(data.play_cost),
    };
    if let Some(cap) = &pred.play_cost_lte { if ref_play_cost > eval_int_constraint(cap, ..) { return false; } }
    if let Some(floor) = &pred.play_cost_gte { if ref_play_cost < eval_int_constraint(floor, ..) { return false; } }
    if let Some(want) = &pred.play_cost_eq { if ref_play_cost != eval_int_constraint(want, ..) { return false; } }
    ```
  - Permanent subject: delegated to the top card's printed cost.
- **YAML shape:**
  - Single comparator: `play_cost: { op: eq, value: 6 }`.
  - Range: `play_cost: [ { op: gte, value: 2 }, { op: lte, value: 4 } ]`.
  - `value` can be a formula. It then compiles to `CompiledDpConstraint::Formula`.
- **Semantics:**
  - The leaf compares the subject's **printed** play cost.
    - For a non-DUAL Option, the cards.json `play_cost` field holds the use cost.
    - A DUAL referenced as an Option (a `kind: option` conjunct, §4-6-6-2) uses the Option use cost, including its self use-cost increase.
    - On the field, the leaf reads the top card's printed cost.
  - Cost reductions or increases applied while paying (§4-2-3 alternate costs) are not reflected.
  - Supported ops are `eq`, `lte` and `gte`. In a list, each op sets one compiled slot, so a duplicated op silently keeps the last value.
  - The legacy keys `play_cost_lte` / `play_cost_gte` parse into the same slots. The lint rejects mixing them with `play_cost:`.
  - A `None` subject gives false.
  - No selection.
- **Rule basis:**
  - §2-6-1: play cost.
  - §2-7-1: use cost (Options).
  - §4-2-3-1: an altered cost is an alternate cost, not the card's play cost.
  - §4-6-5-2 and §4-6-6-2: which information a DUAL card shows.
- **Production users:**
  - **`EX7-008` ToyAgumon** (deck 1-Beelstarmon 3M). This is the **only** production user of the canonical key.
    - Printed: "[On Play] Reveal the top 3 cards of your deck. Add 1 card with [Three Musketeers] in its text and 1 Option card with a cost of 6 among them to the hand."
    - YAML (`cards/ex7/EX7-008.yaml:107`), a reveal-bucket filter:
      ```yaml
      filter: { all_of: [ { kind: option }, { play_cost: { op: eq, value: 6 } } ] }
      ```
  - There is no second user of the canonical key. The nearest legacy-flat sibling is `BT26-005` (`play_cost_lte: 5`; see `trait_contains`).
- **Existing near-miss tests:** about 70 files, none of which is attributed.
  - Examples: `tests/cost_hooks/*`, `tests/dsl/play_cost_budget.rs`, `tests/dsl/predicate_play_or_use_cost_lte.rs`, `tests/dsl/group7_predicate_batch.rs`, `tests/keyword_phase_d/*`.
  - They all contain the bare token `play_cost`: the legacy `play_cost_lte` / `_gte` keys, `play_cost_budget`, `CardData.play_cost`, or helper names.
  - **No test YAML uses the canonical `play_cost:` comparator key.** A grep for `play_cost: {` or `play_cost: [` under `tests/` returns nothing.
- **Proposed behavioural test (for the plan):**
  - Fixture:
    ```yaml
    card: TST-PCOST
    name: Play Cost Probe
    kind: tamer
    color: [yellow]
    cost: 2
    effects:
      - when: on_play
        process:
          - select_hand:
              of: you
              bind_as: eq_pick
              optional: true
              filter: { play_cost: { op: eq, value: 3 } }
              prompt: "Pick a cost-3 card"
          - select_hand:
              of: you
              bind_as: range_pick
              optional: true
              filter: { play_cost: [ { op: gte, value: 3 }, { op: lte, value: 5 } ] }
              prompt: "Pick a cost 3-5 card"
    ```
  - Hand cards: Digimon with cost 2, 3 and 5; an Option with use cost 3; a Digimon with cost 7.
  - Positive cases:
    - The first prompt offers 2 cards: the cost-3 Digimon and the cost-3 Option.
    - Decline it with PASS. The second prompt then offers 3 cards: the cost-3 Digimon, the cost-3 Option and the cost-5 Digimon.
  - Negative cases:
    - Costs 2 and 7 are never offered.
    - A third clause with `play_cost: { op: lte, value: 1 }` gives no prompt at all.
    - Permanent-subject reading uses the top card's cost, not the sum. Build a P0 stack whose top card costs 5 over a cost-3 source. It satisfies `any_permanent { play_cost: { op: eq, value: 5 } }` and fails `eq 3`.
  - Selection surface: an optional pending_selection, with no auto-pick.
  - Optional extra: put a cost-0 Digi-Egg in the trash and run `select_trash { filter: { play_cost: { op: lte, value: 5 } } }`. This documents defect 1.
- **Suspected engine/DSL defects (read-only):**
  1. **Cards with no printed play cost read as 0.**
     - For Digi-Eggs, cards.json stores cost 0 as a missing-value default (see the BT26-005 YAML comment).
     - So `play_cost: {op: lte, value: N}` matches Digi-Eggs.
     - The CR says a Digi-Egg's information has no play cost (cf. §1-4-4-5 for tokens; DCGO `HasPlayCost`).
     - Authors must add a guard by hand.
  2. **The replacement-subject check misses some cost and identity leaves.**
     - `predicate_reads_replacement_subject` (`lower_replacement.rs:448-497`) lists `play_cost_lte`.
     - It does not list `play_cost_gte` or `play_cost_eq`, nor `trait_contains` or `self_color_count_gte`.
     - A replacement `active_when` that uses only those leaves is not treated as reading the replacement subject, so the replacement is restricted to the carrier itself (`lower_replacement.rs:140, 158`).

---

### `predicate:source_count`

- **Lowering site(s):**
  - Parse: DSL `predicate.rs:249`. The payload is `SourceCountPredicate { filter: Box<PredicateSpec>, at_least: u8 }` (`predicate.rs:1458-1470`). Compiled via `compile.rs:1042`.
  - Runtime, Permanent subject: ENG `predicate.rs:4504-4529`. Breeding-area subject: `predicate.rs:4859-4881`.
    ```rust
    if let Some((filter, at_least)) = &pred.source_count {
        let n = perm.card_sources.len();
        let matching = if n <= 1 { 0 } else {
            perm.card_sources[..n - 1].iter()
                .filter(|s| eval_predicate_ref(filter, rctx,
                        PredicateSubject::Card(s.handle()), bindings, DualRef::Digimon))
                .count()
        };
        if matching < usize::from(*at_least) { return false; }
    }
    ```
  - The leaf is **not** evaluated for Card or `None` subjects, where it passes vacuously.
- **YAML shape:**
  ```yaml
  source_count:
    filter: { name_contains: "Vemmon" }
    at_least: 2
  ```
- **Semantics:**
  - The leaf counts the digivolution cards strictly below the top card (index 0 is the bottom) that satisfy the nested filter.
  - Each source is evaluated as a **Card** subject, using the full combinator evaluator.
  - The leaf is true when the count is at least `at_least`. `at_least: 0` is always true.
  - Link cards are not counted. `link_card_count` counts those.
  - A DUAL card in the sources is read as an Option when the filter says `kind: option` (§4-6-6-2).
  - **Face-down sources are evaluated on their printed data.**
  - Source-only leaves inside the filter (`is_face_down`, `is_bottom_source`) are always false, because the subject is `Card`, not `Source`.
  - On a `None` or Card subject the leaf is ignored, so it is vacuously true.
  - No selection.
- **Rule basis:**
  - §4-7-7: bottom cards are part of the top card's information.
  - §4-7-9: a face-down card has no card information and can't be referenced.
  - §4-9-2: a link card isn't a stacked card.
  - §4-6-6-2: a DUAL card in the digivolution cards is read as an Option.
- **Production users:**
  - **`BT25-092` Asuna Shiroki** (deck 1-Beelstarmon 3M).
    - Printed: "[Main] By suspending this Tamer and trashing 1 Option card from your hand or your Digimon's digivolution cards, 1 of your Digimon may digivolve ..."
    - YAML (`cards/bt25/BT25-092.yaml:102`), used as a payability gate:
      ```yaml
      any_permanent: { of: you, zone: [battle_area], kind: digimon, source_count: { filter: { kind: option }, at_least: 1 } }
      ```
  - **`P-094` Destromon.**
    - Printed (inherited): "... by placing 2 [Vemmon] from 1 of your [Galacticmon]'s digivolution cards at the bottom of their owners' decks, switch the target of attack to this Digimon."
    - YAML (`cards/p/P-094.yaml:184-205`), in both the `condition` and the `select_own_permanent` filter:
      ```yaml
      all_of: [ { name_contains: "Galacticmon" }, { source_count: { filter: { name_contains: "Vemmon" }, at_least: 2 } } ]
      ```
  - `BT25-085` BeelStarmon (same deck) uses it together with `link_card_count`.
- **Existing near-miss tests:** none of these exercises the leaf. The `src/` hits are production source.
  - `tests/dsl/predicate_leaves_ii.rs` exercises a *different* leaf: the `event_target_*` family with `source_count_just_before` (deletion snapshot).
  - `tests/effect_context/opponent_stack_trashing.rs` has a helper fn *named* `source_count`, used as a select predicate closure. Name collision only.
- **Proposed behavioural test (for the plan):**
  - Fixture:
    ```yaml
    card: TST-SRCCOUNT
    name: Source Count Probe
    kind: tamer
    color: [purple]
    cost: 2
    effects:
      - when: on_play
        process:
          - select_own_permanent:
              bind_as: pick
              optional: true
              filter:
                kind: digimon
                source_count: { filter: { name_contains: "Vemmon" }, at_least: 2 }
              prompt: "Pick a Digimon with 2+ Vemmon sources"
    ```
  - Setup (P0 battle area):
    - A = stack [Vemmon, Vemmon, Galacticmon]: 2 matching sources.
    - B = [Vemmon, Galacticmon]: 1 matching source.
    - C = a Galacticmon whose top card is named "Vemmon" and which has 1 Vemmon source. The top card is not counted, so its count is 1.
    - D = [Vemmon, Vemmon (face_down = true), Galacticmon]. See defect 1.
  - Positive cases:
    - The prompt offers A.
    - **Today D is also offered.** Pin whichever behaviour the plan decides, then follow up on the defect.
  - Negative cases:
    - B and C are not offered.
    - With only B and C on the field, there is no pending_selection.
    - Also cover `at_least: 0` with a bare Digimon, which is offered.
  - Selection surface: an optional pending_selection offering exactly the expected permanents.
  - Second test: put a DUAL card in the sources and use `filter: { kind: option }`. It is counted (§4-6-6-2).
- **Suspected engine/DSL defects (read-only):**
  1. **Face-down sources are counted by their printed identity.**
     - §4-7-9 says face-down cards have no information.
     - A filter cannot exclude them either: `is_face_down` inside the nested filter is always false, because the subject is `Card` (`predicate.rs:181-188`).
  2. **The leaf is silently ignored on `None` and Card subjects.**
     - It is absent from `eval_no_subject_fields` (`predicate.rs:1529`).
     - So `if: { condition: { source_count: ... } }` is always true.
  3. **P-094's gate counts sources a later return could not move.** Stack-return protection is not considered. This is a minor §15-7-4 exposure.

---

### `predicate:self_color_count_gte`

- **Lowering site(s):**
  - Parse: DSL `predicate.rs:329` (`Option<u8>`). Compiled form: `compiled.rs:503`, `compile.rs:1099`.
  - Runtime, Card subject: ENG `predicate.rs:3411-3415`. It uses `ref_colors` (`3218-3222`); for a DUAL card these are the Digimon colours unless `kind: option`.
  - Runtime, Permanent subject: `predicate.rs:4454-4458`, against `synth_identity.colors`. The delegated printed check is stripped (`4244-4256`, `4316-4318`).
    ```rust
    if let Some(floor) = pred.self_color_count_gte {
        if distinct_color_count(&synth_identity.colors) < usize::from(floor) { return false; }
    }
    ```
  - `distinct_color_count` (`predicate.rs:3898`) de-duplicates.
  - `synth_identity` applies `ChangeBaseCardColor` (replace) first and then `AddColor` (append). For `AddColor` with `ModifierPayload::None`, it adds the colours of every **non-face-down** source (`permanent.rs:330-356`).
- **YAML shape:** `self_color_count_gte: 2`. It appears in a digivolve `from:` filter, e.g. `from: { level_eq: 6, self_color_count_gte: 2 }`, or in an aura `active_when`.
- **Semantics:**
  - The leaf is true when the subject has at least N distinct colours.
  - For a hand or trash card it reads the printed colours (the DUAL Digimon side by default).
  - For a field permanent it reads the effective colours after colour-changing and colour-adding effects.
  - There is no "colourless" `CardColor`, so a colourless card counts as 0.
  - A `None` subject gives false.
  - No selection.
- **Rule basis:**
  - §2-4-3 and §2-4-3-1: a multicolour card has all of its colours.
  - §15-12-2: information-modifying effects apply. Compare the BT8-084 official Q&A on colours gained from sources.
  - §4-7-9: face-down sources contribute no colour.
- **Production users:**
  - **`EX12-076` Susanoomon** (deck 7-Toho Braves).
    - Printed digivolve requirement (per the YAML alt_path): "Lv.6 w/2 or more colors: Cost 6".
    - YAML (`cards/ex12/EX12-076.yaml:14`):
      ```yaml
      alt_paths:
        - kind: digivolve
          from: { level_eq: 6, self_color_count_gte: 2 }
          cost: 6
      ```
  - **`BT8-084` Kimeramon.**
    - Printed: "[Your Turn] This Digimon is treated as also having the color of its digivolution cards. While this Digimon has 4 or more color, it gets +4000 DP."
    - YAML (`cards/bt8/BT8-084.yaml:192`):
      ```yaml
      - kind: aura
        target: {}
        active_when: { your_turn: true, self_color_count_gte: 4 }
        dp_modifier: 4000
      ```
- **Existing near-miss tests:** none. `src/dsl_cards/predicate.rs` is source only.
- **Proposed behavioural test (for the plan):**
  - Fixture:
    ```yaml
    card: TST-COLORS
    name: Color Count Probe
    kind: digimon
    level: 4
    color: [red]
    cost: 4
    dp: 4000
    effects:
      - kind: aura
        target: {}
        active_when: { all_turns: true, self_color_count_gte: 3 }
        dp_modifier: 1000
      - when: on_play
        process:
          - select_hand:
              of: you
              bind_as: pick
              optional: true
              filter: { self_color_count_gte: 2 }
              prompt: "Pick a 2+ colour card"
    ```
  - Hand cards: mono red, red/blue, red/blue/yellow. If `make_test_card` allows it, add a card listed as `[red, red]` to check de-duplication.
  - Positive case, Card subject: the prompt offers 2 cards (the 2-colour and 3-colour ones).
  - Positive case, Permanent subject:
    - Place TST-COLORS on P0's field with blue and yellow sources.
    - Install `ModifierEntry::simple(ModifierType::AddColor, 0, Expiry::Permanent, 0)`. Its payload is None, meaning "colours of its digivolution cards".
    - Tick declaratives. `runner.effective_dp(h)` should be 5000 (red + blue + yellow = 3 colours).
  - Negative cases:
    - Without AddColor, DP stays 4000 (printed red only).
    - With AddColor but the yellow source set `face_down = true`, the count is 2 and DP stays 4000.
    - The mono card is never offered.
    - The duplicate-colour card counts as 1 colour.
  - Selection surface: an optional pending_selection.
- **Suspected engine/DSL defects (read-only):** none seen. The Card path ignores `ChangeBaseCardColor` and `AddColor`, which only exist on permanents. That is correct.

---

### `predicate:face_down_sources_under_tamers_gte`

- **Lowering site(s):**
  - Parse: DSL `predicate.rs:365` (`Option<u8>`). Compiled form: `compiled.rs:519`, `compile.rs:1115`.
  - Runtime: ENG `predicate.rs:497-507`. This is a subject-independent global leaf and always reads `rctx.player`:
    ```rust
    if let Some(floor) = pred.face_down_sources_under_tamers_gte {
        let total = crate::dsl_cards::step::selections::face_down_sources_under_tamers(rctx, rctx.player);
        if total < usize::from(floor) { return false; }
    }
    ```
  - The count, `face_down_sources_under_tamers` (`step/selections.rs:6909-6927`):
    - It takes the controller's battle-area permanents that pass `kind: tamer` (the effective, synth kind).
    - It sums `face_down_cards_under(game, handle, controller)` over them (`step/selections.rs:7110-7130`):
      ```rust
      perm.face_down_source_indices()
          .take_while(|&i| !game.source_card_trash_blocked(handle, perm.card_sources[i].handle(), controller))
          .count()
      ```
    - `face_down_source_indices` (`permanent.rs:202`) yields face-down sources below the top card, **bottom first**.
- **YAML shape:** `face_down_sources_under_tamers_gte: 2`, usually in `if: { condition: ... }` or in a clause `condition:`.
- **Semantics:**
  - The leaf is true when the controller's battle-area Tamers together hold at least N face-down digivolution cards **that this effect could actually trash, taking each Tamer's lowest face-down card first**.
  - Per Tamer, counting stops at the first face-down card protected from this controller's trash. Two protections count: `ImmuneFromStackTrashing` (scoped by controller) and a `CannotTrash` `ProtectedSourceCards` payload.
  - This is the same count the `trash_bottom_face_down_sources_under_tamers` cost uses, so the gate matches §15-7-4 payability.
  - It excludes: face-up sources, face-down cards under Digimon, breeding-area permanents, and the opponent's Tamers.
  - It works with any subject, including `None`.
  - No selection.
- **Rule basis:**
  - §15-7-4 (Ver. 4.3, REVERSED): an optional processing condition that can't be executed can't be chosen. This leaf is the payability gate. See `docs/digimon-rules/changelog-3.6-to-4.3.md`, row §15-7-4.
  - §15-7-3: no partial payment.
  - §4-7-9: face-down cards.
  - Owner ruling 19 (2026-10, commit 149e3abc0): "bottom face-down card" means the lowest face-down card.
- **Production users:**
  - **`BT26-082` Ravemon** (deck 10-DATA SQUAD Ravemon).
    - Printed: "[When Digivolving] [End of Attack] By deleting this Digimon or trashing 2 bottom face-down cards from under any of your Tamers, delete 1 of your opponent's highest DP Digimon."
    - YAML (`cards/bt26/BT26-082.yaml:70`):
      ```yaml
      - if: { condition: { face_down_sources_under_tamers_gte: 2 }, then: [ select_effect_choice: {...} ] }
      ```
  - **`ST24-10` Lilamon** (same deck).
    - Printed: "Then, by trashing 2 bottom face-down cards from under any of your Tamers, this Digimon may digivolve into a [DATA SQUAD] trait Digimon card in the hand without paying the cost."
    - YAML (`cards/st24/ST24-10.yaml:91`):
      ```yaml
      if: { condition: { all_of: [ { face_down_sources_under_tamers_gte: 2 }, { count_gte: { ... DATA SQUAD in hand } } ] } }
      ```
- **Existing near-miss tests:** none. `src/dsl_cards/predicate.rs` is source only.
- **Proposed behavioural test (for the plan):**
  - Fixture:
    ```yaml
    card: TST-FDTAMER
    name: Face-down Under Tamer Probe
    kind: digimon
    level: 3
    color: [purple]
    cost: 3
    dp: 3000
    effects:
      - when: on_play
        process:
          - if:
              condition: { face_down_sources_under_tamers_gte: 2 }
              then:
                - gain_memory: 1
    ```
  - Setup:
    - Two P0 Tamers, T1 and T2 (`make_test_card` with `card_kind = Tamer`).
    - Push sources and set their `face_down` flags as each case needs.
    - Memory starts at 0.
  - Positive cases:
    - T1 with 1 face-down card + T2 with 1 face-down card → memory +1 (the count spans Tamers).
    - T1 with 2 face-down cards → memory +1.
    - T1 with 2 face-down cards plus an `ImmuneFromStackTrashing` that P1 installed with an `OpponentOnly` filter → still counts 2 for P0's own effect → memory +1.
  - Negative cases (each gives no memory gain):
    - T1 with 1 face-down + 1 face-up card.
    - A P0 *Digimon* with 3 face-down sources.
    - P1's Tamer with 3 face-down sources.
    - T1 with 2 face-down cards, where the **bottom** face-down card has a `CannotTrash` `ProtectedSourceCards([that card])` modifier. T1's count is 0.
    - The same, but with the *upper* face-down card protected. T1's count is 1.
  - Selection surface: none (pure gate).
- **Suspected engine/DSL defects (read-only):**
  - None in the leaf itself.
  - Side note, not about this construct: ST24-10 additionally gates on a [DATA SQUAD] card being in hand, "so the cost is never paid for nothing". §15-7-5 lets the player pay even when the follow-up can't happen, so that extra gate removes a legal choice. Worth a ticket.

---

### `predicate:on_field`

- **Lowering site(s):**
  - Parse: DSL `predicate.rs:631` (`Option<bool>`). Compiled form: `compiled.rs:626`, `compile.rs:1303`.
  - Runtime: ENG `predicate.rs:771-783`. It is evaluated before subject dispatch, so it works for every subject:
    ```rust
    if let Some(want) = pred.on_field {
        let is_on_field = match subject {
            PredicateSubject::Permanent(h) => h.index != crate::action::space::BREEDING_TARGET as u8,
            _ => rctx.source_permanent.is_some_and(|h| h.index != crate::action::space::BREEDING_TARGET as u8),
        };
        if is_on_field != want { return false; }
    }
    ```
- **YAML shape:** `on_field: true`, inside a clause `condition:` (`all_of`).
- **Semantics:**
  - Despite the name, the leaf means **"in the battle area"**.
  - It is true when the subject permanent's handle is a battle-area handle rather than the breeding sentinel. For non-permanent subjects it checks the effect's `source_permanent` instead.
  - A card in hand, trash or security has no `source_permanent`, so the leaf is false.
  - The breeding area gives false, even though under the CR the breeding area *is* part of the field (§3-4-6).
  - The handle is not re-validated, so a stale `source_permanent` still reads true (see defect 2).
  - No selection.
- **Rule basis:**
  - §3-4-6: the field consists of the breeding area and the battle area.
  - Otherwise this is a card-text idiom. An Option placed in the battle area by `<Delay>` (BT21-093) must only use its "[All Turns]" text while it is there.
- **Production users:**
  - **`BT21-093` Raging Serpentine** (deck 9-Medusamon). This is the **only** production user.
    - Printed: "[Main] Delete 1 of your opponent's highest DP Digimon. Then, place this card in the battle area. [All Turns] When your opponent's security stack is removed from, ＜Delay＞ ..."
    - YAML (`cards/bt21/BT21-093.yaml:168`):
      ```yaml
      when: on_opponent_security_removed
      active_when: { all_turns: true }
      condition: { all_of: [ { on_field: true }, { any_field_permanent: {...} } ] }
      ```
  - There is no second user.
- **Existing near-miss tests:**
  - `tests/rules_conformance/ch15_15_unaffected.rs` defines its own helper `fn on_field(r, player, card_id) -> bool`, which checks whether a card is present. Token collision; it never compiles the leaf.
  - `src/dsl_cards/formula_eval.rs` lists `on_field` among the "permanent-only leaves" that card-zone filters do not support.
- **Proposed behavioural test (for the plan):**
  - Fixture:
    ```yaml
    card: TST-ONFIELD
    name: On Field Probe
    kind: digimon
    level: 3
    color: [red]
    cost: 3
    dp: 3000
    effects:
      - when: on_play
        condition: { on_field: true }
        process:
          - gain_memory: 1
      - when: on_play
        process:
          - if:
              condition: { on_field: false }
              then:
                - gain_memory: 2
    ```
  - Positive case:
    - Place the probe on P0's battle area and fire OnPlay. Memory rises by exactly 1.
    - The first clause fires.
    - The `if` does not add 2: its subject is `None`, so it reads `source_permanent`, which is in the battle area.
  - Negative cases:
    - **Breeding area.** Breeding-area triggers are only collected for breeding-zone-flagged clauses, so firing through the queue is not a fair negative. Instead:
      - Call `place_in_breeding`.
      - Build an `EffectReadContext` whose `source_permanent` is the breeding sentinel handle (`index = BREEDING_TARGET`).
      - Evaluate the compiled predicate directly: `eval_predicate(on_field: true)` returns false.
      - The test still gets credit if the YAML literal is the fixture.
    - **Off the field.** Write an Option fixture with `when: main_from_hand` and the same `if`, and use it from hand. With `source_permanent = None` it takes the `on_field: false` branch.
  - Selection surface: none.
- **Suspected engine/DSL defects (read-only):**
  1. **The name does not match the CR.** The leaf means "battle area", but §3-4-6 defines the field as breeding area + battle area. Either rename it or document it.
  2. **Stale handle.**
     - After a deletion, `predicate_subject_for_source` drops to `None`, but `rctx.source_permanent` still holds the old handle.
     - So `on_field: true` stays true for a card that has left the field.
     - The leaf never checks that the slot still holds the card (`predicate.rs:774-778`).
     - The same applies whenever a source permanent leaves mid-resolution.

---

### `predicate:can_attack`

- **Lowering site(s):**
  - Parse: DSL `predicate.rs:310`. The payload is `CanAttackPredicate` (`predicate.rs:1497-1510`):
    - `targets: AttackTargetSpec` (`any` | `player` | `digimon`)
    - `without_suspending: bool`
    - `ignore_summoning_sickness: bool`
  - Compiled form: `compiled.rs:491`, `CompiledCanAttack` (`compiled.rs:3250`), `compile.rs:1091-1095`.
  - Runtime: ENG `predicate.rs:4393-4423`.
    - Permanent subject only.
    - Breeding-area subject → false via line `4952`.
    - Card and `None` subjects → ignored, so vacuously true.
    ```rust
    if let Some(spec) = pred.can_attack {
        if rctx.game.turn_player() != handle.player { return false; }
        let restriction = match spec.targets { Any => AttackTargetRestriction::Any,
            Player => PlayerOnly, Digimon => DigimonOnly };
        let can = !crate::action::mask::effect_attack_target_action_ids_with_options(
            rctx.game, handle, restriction, spec.without_suspending, spec.ignore_summoning_sickness).is_empty();
        if !can { return false; }
    }
    ```
  - Target enumeration: `attack_declaration_action_ids` (`action/mask.rs:989-1074`), called with `vortex = false` and `unsuspended_targets = false`.
  - Attacker legality: `Game::can_attack` (`combat/mod.rs:344`), `can_attack_inner` (`:401`) and `can_attack_without_suspending_inner` (`:438`).
- **YAML shape:**
  - `filter: { kind: digimon, can_attack: { targets: any } }` (BT24-037).
  - `filter: { kind: digimon, can_attack: { without_suspending: true } }` (EX13-077).
  - Other options: `ignore_summoning_sickness: true`, and `targets: player` or `targets: digimon`.
- **Semantics: the exact engine states read.** The leaf is true iff **all** of the following hold:
  1. **Turn:** `turn_player == handle.player` (`predicate.rs:4400`). From the attacker's controller's point of view, the leaf is never true on the opponent's turn.
  2. **No player-scoped `CannotAttack`** on the controller (`mask.rs:998-1003`).
  3. **No permanent-scoped `CannotAttack`** (`mask.rs:1007`). Modifiers are read through the read-time immunity gate, so a `CannotAttack` hidden by an immunity does not count.
  4. **It is a Digimon for the rules** (`is_digimon_for_rules`). Tamers and Options are false; tokens and DUAL top cards are true.
  5. **No `CannotSuspend`.** This is enforced even with `without_suspending: true` (`combat/mod.rs:458-460`, citing §11-2-5).
  6. **Not suspended**, unless `without_suspending` is set. `can_attack_without_suspending_inner` never reads `is_suspended`.
  7. **Summoning sickness:** `turn_played == turn_count` blocks the attack, unless the Digimon has `Rush` (printed, or granted via `has_keyword`) or `ignore_summoning_sickness` is set.
  8. **At least one legal target exists.** A legal target is either:
     - the player, when `targets` is `any` or `player` and the attacker has no `CannotAttackPlayer`; or
     - for `any` or `digimon`, an opponent battle-area Digimon (`permanent_is_digimon_for_rules`) that is suspended. If the attacker has `CanAttackUnsuspended`, any opponent Digimon qualifies.

     In both cases the target must not be blocked by `attack_target_blocked_by_modifier`. That function checks:
     - the player modifier `MayAttackPlayerOnly`;
     - a `CannotAttackTarget` on the target (overridden by `CanAttackTargetDefendingPermanent`);
     - `CannotBeAttackedBySecurityAttackChanged` on the target, combined with a Security A. change on the attacker.

  **Not read:**
  - `pending_attack`, i.e. whether an attack is in progress (§11-2-4)
  - memory
  - `ForceAttack`
  - the current phase
  - `<Raid>` / `<Execute>` (§16-38)

  **Expiry:** expiry matters only indirectly. Expired modifiers are no longer in the registry, so the leaf flips once `Expiry::EndOfTurn` / `EndOfOpponentsTurn` entries are purged.

  **Selection:** the leaf surfaces no selection by itself. As a select filter, it decides which permanents the (real, optional) pick offers. An empty set means no prompt.
- **Rule basis:**
  - §11-1-2: only the turn player can attack.
  - §11-2-1 and §11-2-8-1: suspend the Digimon to declare an attack.
  - §11-2-5: a Digimon that can't suspend can't declare an attack.
  - §11-2-7-1: attack targets are the opponent or a suspended Digimon.
  - §1-3-1 example + §16-15-1 `<Rush>`: Digimon played this turn.
  - §15-1-3: prohibition beats enabling.
  - §11-2-4: no new declaration during an attack. Not modelled.
- **Production users:**
  - **`BT24-037` Silphymon** (decks 3-Jupitermon, 8-Jupitermon, 4-TS Toolbox and 5-Chronomon).
    - Printed: "... Then, 1 of your Digimon may attack."
    - YAML (`cards/bt24/BT24-037.yaml:53`):
      ```yaml
      - select_own_permanent: { bind_as: attacker, optional: true, filter: { kind: digimon, can_attack: { targets: any } } }
      - may_attack_now: { attacker: attacker, targets: any }
      ```
  - **`EX13-077` Omnimon: Merciful Mode.**
    - Printed: "[On Play] [When Digivolving] 1 of your Digimon may attack without suspending."
    - YAML (`cards/ex13/EX13-077.yaml:89`): the filter is `{ kind: digimon, can_attack: { without_suspending: true } }`, followed by `may_attack_now { without_suspending: true }`.
- **Existing near-miss tests:** no test touches the leaf. The token appears only in source:
  - `src/dsl_cards/formula_eval.rs`, in the list of leaves card-zone filters do not support.
  - `src/dsl_cards/lower_replacement.rs`, in the subject-reading list.
  - `src/dsl_cards/predicate.rs`.
- **Proposed behavioural test (for the plan):** a table-driven test, with one fresh runner per row.
  - Fixture:
    ```yaml
    card: TST-CANATK
    name: Can Attack Probe
    kind: tamer
    color: [yellow]
    cost: 2
    effects:
      - when: on_play
        process:
          - select_own_permanent:
              bind_as: attacker
              optional: true
              filter: { kind: digimon, can_attack: { targets: any } }
              prompt: "1 of your Digimon may attack"
      - when: on_play
        process:
          - select_own_permanent:
              bind_as: free_attacker
              optional: true
              filter: { kind: digimon, can_attack: { targets: digimon, without_suspending: true } }
              prompt: "attack a Digimon without suspending"
    ```
  - Setup:
    - P0 is the turn player: `set_first_player(0)`, `turn_count ≥ 1`, Main phase.
    - The carrier TST-CANATK is on P0's field.
    - The subject Digimon D is on P0's field.
    - Opponent Digimon are placed on P1's field as each row needs.
    - Fire OnPlay. Assert `valid_action_ids.len()` of the first prompt, then PASS and check the second prompt.

    | row | state of D / board | first prompt (`targets: any`) | second prompt (`digimon`, w/o suspending) |
    |---|---|---|---|
    | 1 | unsuspended, `place_on_field(..,Some(0))`, no opp Digimon | offered | not offered (no Digimon target) |
    | 2 | as row 1, opp has 1 suspended Digimon | offered | offered |
    | 3 | D suspended, opp has a suspended Digimon | not offered | **offered** (suspension ignored) |
    | 4 | D fresh (`None` turn), no Rush | not offered | not offered |
    | 5 | D fresh + `grant_keyword Rush` | offered | offered (opp suspended) |
    | 6 | D has a permanent `CannotAttack` | not offered | not offered |
    | 7 | P0 has a player-scoped `CannotAttack` | not offered (no prompt at all) | not offered |
    | 8 | D has `CannotSuspend` | not offered | **not offered** (see defect 1) |
    | 9 | D has `CannotAttackPlayer`, no opp Digimon | not offered | not offered |
    | 10 | opp has only unsuspended Digimon; D has `CanAttackUnsuspended` | offered | offered |
    | 11 | the opp's suspended Digimon carries `CannotAttackTarget` | offered (via the player target) | not offered |
    | 12 | P1 is the turn player (carrier fired on P1's turn) | no prompt | no prompt |
    | 13 | D is a Tamer | not offered | not offered |

  - Positive and negative cases are covered by the table. Each "not offered" row with no other candidate asserts `pending_selection.is_none()`.
  - Selection surface: whenever a permanent is offered, `is_optional == true` (matching "may"), and there is no auto-pick with 1 candidate.
- **Suspected engine/DSL defects (read-only):**
  1. **`without_suspending` combined with `CannotSuspend` diverges from official Q&A Q3185** (BT19-101, 2024-09-20).
     - The Q&A says a Digimon that "can't suspend" **can** attack via an attack-without-suspending effect. This matches DCGO `withoutTap`.
     - The engine refuses it (`combat/mod.rs:452-460`), citing §11-2-5 over DCGO.
     - The official Q&A is the more specific Bandai ruling, so this needs an owner decision.
  2. **§11-2-4 is not consulted.** That rule forbids a new attack declaration during an attack.
     - A "may attack" pick resolved while `pending_attack` is live (e.g. from a [When Digivolving] effect during an attack) would still offer candidates.
     - Check whether `may_attack_now` refuses them later.
  3. **The leaf is silently ignored on `None` and Card subjects.** For example, `if: { condition: { can_attack: {} } }` is always true. This is an omission in `predicate.rs:1529`.

---

### `predicate:can_change_orientation`

- **Lowering site(s):**
  - Parse: DSL `predicate.rs:284` (`Option<bool>`). Compiled form: `compiled.rs:482`, `compile.rs:1088`.
  - Runtime: ENG `predicate.rs:4379-4391`.
    - Permanent subject only.
    - Breeding-area subject → false via line `4949`.
    - Card and `None` subjects → ignored.
    ```rust
    if let Some(want) = pred.can_change_orientation {
        let can = if perm.is_suspended {
            !rctx.game.cannot_unsuspend(handle)
        } else {
            !rctx.game.modifiers.has(handle, crate::enums::ModifierType::CannotSuspend)
        };
        if can != want { return false; }
    }
    ```
  - `Game::cannot_unsuspend` (`game/suspend.rs:156-173`) is true in either of two cases:
    - the permanent has a permanent-scoped `CannotUnsuspend`; or
    - the controller has a **player-scoped** `CannotUnsuspend` and the permanent is a battle-area Digimon. The player-scoped form applies to Digimon only (G-PLAYER-MASS-CANNOT-UNSUSPEND).
- **YAML shape:** `can_change_orientation: true`. It is usually paired with `is_unsuspended: true` in the filter of a "by suspending 1 of your ..." cost.
- **Semantics:**
  - This mirrors DCGO `Permanent.CanChangeOrientation`: a suspended permanent must be able to unsuspend, and an unsuspended one must be able to suspend.
  - States read:
    - `is_suspended`
    - permanent-scoped `CannotSuspend`
    - permanent-scoped `CannotUnsuspend`
    - player-scoped `CannotUnsuspend` (Digimon only)
  - States not read:
    - Reboot and the unsuspend phase
    - "can't be suspended by opponent's effects" variants (no such modifier exists)
  - There is no player-scoped `CannotSuspend` in the engine. A grep finds only permanent-scoped `has(...)` sites.
  - `can_change_orientation: false` selects the complement.
  - The leaf surfaces no selection by itself.
- **Rule basis:**
  - §4-13: card orientation.
  - §15-1-3: prohibition beats enabling.
  - §15-7-4 (Ver. 4.3): an unpayable "by suspending" condition can't be chosen. This leaf is what keeps such a cost from being offered.
  - §11-2-5 (related): a Digimon that can't suspend can't attack.
- **Production users:**
  - **`EX13-024` Slayerdramon** (deck 6-Examon).
    - Printed: "[All Turns] [Once Per Turn] When any of your [Dracomon] or [Examon] text Digimon would leave the battle area, by suspending 1 of your such Digimon, they don't leave."
    - YAML (`cards/ex13/EX13-024.yaml:148-170`), in `active_when`, plus the `select_own_permanent` cost filter with the same leaves:
      ```yaml
      any_permanent: { of: you, zone: [battle_area], kind: digimon, is_unsuspended: true, can_change_orientation: true, any_of: [...] }
      ```
  - **`EX3-024` Slayerdramon** (deck 7-Toho Braves).
    - Printed: "[Start of Opponent's Main Phase] By suspending 1 of your Digimon with [Dramon] or [Examon] in its name, your opponent attacks with 1 of their Digimon."
    - YAML (`cards/ex3/EX3-024.yaml:39`):
      ```yaml
      select_own_permanent: { filter: { kind: digimon, is_unsuspended: true, can_change_orientation: true, any_of: [...] }, optional: true }
      ```
- **Existing near-miss tests:** none. The token appears only in source: `src/dsl_cards/formula_eval.rs`, `src/dsl_cards/lower_replacement.rs` and `src/dsl_cards/predicate.rs`.
- **Proposed behavioural test (for the plan):**
  - Fixture:
    ```yaml
    card: TST-ORIENT
    name: Orientation Probe
    kind: tamer
    color: [blue]
    cost: 2
    effects:
      - when: on_play
        process:
          - select_own_permanent:
              bind_as: can
              optional: true
              filter: { can_change_orientation: true }
              prompt: "can change orientation"
          - select_own_permanent:
              bind_as: cannot
              optional: true
              filter: { can_change_orientation: false }
              prompt: "cannot change orientation"
    ```
  - Setup (P0 permanents):
    - U: an unsuspended Digimon.
    - US: an unsuspended Digimon with `CannotSuspend`.
    - S: a suspended Digimon.
    - SU: a suspended Digimon with a permanent-scoped `CannotUnsuspend`.
    - SP: a suspended Digimon, with P0 carrying a **player-scoped** `CannotUnsuspend`.
    - ST: a suspended Tamer under the same player modifier.
    - Use separate runners for the player-modifier rows.
    - The carrier TST-ORIENT is itself an unsuspended Tamer, so it counts in the "can" set. Account for it, or give it `CannotSuspend`.
  - Positive cases:
    - The "can" prompt offers U, S and ST, plus the carrier.
    - The "cannot" prompt offers US, SU and SP.
  - Negative cases:
    - A breeding-area Digimon is never in either set (the breeding eval returns false).
    - If every P0 permanent is "cannot", the first select shows no prompt.
  - Selection surface: an optional pending_selection with the exact ids.
- **Suspected engine/DSL defects (read-only):**
  1. **The leaf is silently ignored on `None` and Card subjects** (`predicate.rs:1529`).
  2. **EX13-051 Guardromon's gate is incomplete.**
     - The card (a `suspend_and_cancel_all_matching_leaves` user) gates only on `source_is_unsuspended`, not on `can_change_orientation`.
     - With `CannotSuspend` on the carrier, the optional replacement is offered but cannot be paid (§15-7-4).
     - See the last section.

---

### `verb:grant_narrow_opponent_effect_protection`

- **Lowering site(s):**
  - Parse: DSL `step.rs:436` (`StepSpec::GrantNarrowOpponentEffectProtection(GrantNarrowOpponentEffectProtectionArgs)`). The args are `{ target: BindingRef, expiry: String }` (`step.rs:3365-3376`, `deny_unknown_fields`). Compiled form: `compiled.rs:2428`, `compile.rs:3869-3873`.
  - Step: ENG `dsl_cards/step/modifiers.rs:446-457`.
    - An unknown `expiry` is a no-op, with a debug eprintln.
    - A non-permanent binding is a no-op.
  - Engine: `effect_context/action/modifiers.rs:489-513`:
    ```rust
    if !self.can_affect_permanent(target) { return; }
    self.game.modifiers.add(target,
        ModifierEntry::simple(ModifierType::ImmuneFromDPMinus, 0, expiry, self.player)
            .with_effect_immunity_filter(EffectImmunityFilter {
                source_kind: None, controller: EffectControllerFilter::OpponentOnly }));
    self.game.modifiers.add(target,
        ModifierEntry::passive_replacement(ModifierType::CannotBeDeDigivolved, expiry, self.player));
    self.game.mark_until_condition_dirty();
    ```
  - Consult sites:
    - **`ImmuneFromDPMinus`** is read by `Game::effective_dp` (`combat/dp.rs:46-90`).
      - It drops a negative `ChangeDp` entry when `entry.source_player != handle.player`.
      - So "opponent" is decided against the *protected permanent's controller*, at read time.
    - **`CannotBeDeDigivolved`** is a passive replacement.
      - Its `default_passive_cause_filter` is `OpponentEffect` (`modifiers.rs:585-599`).
      - It maps to `EffectTiming::WhenWouldBeDeDigivolved` (`replacement.rs:938`).
      - `de_digivolve_core` fires that timing once (`game_actions/helpers.rs:307-314`), with `cause = infer_effect_cause(target.player)`.
- **YAML shape:**
  ```yaml
  - grant_narrow_opponent_effect_protection:
      target: target          # or `source`
      expiry: end_of_opponents_turn
  ```
- **Semantics:**
  - The verb installs two modifiers on one permanent until `expiry`:
    - **DP:** `effective_dp` ignores negative `ChangeDp` deltas whose source player is the protected Digimon's opponent.
    - **De-Digivolve:** any De-Digivolve whose replacement cause is `OpponentEffect` is cancelled.
  - The grant itself is gated by `can_affect_permanent`. An own effect on an own Digimon is almost always allowed.
  - The verb surfaces no selection. The target pick happens in a preceding `select_own_permanent`, which is mandatory in BT16-055 and BT26-029.
- **Paths the grant MUST block** (opponent-controlled effects only), with engine status:

  | # | path (opponent effect on the protected Digimon) | printed? | engine today |
  |---|---|---|---|
  | B1 | `add_dp_modifier` with a negative value from an opponent card's effect (Digimon, Tamer, Option, inherited or linked) | yes | **blocked** (`ChangeDp` with `source_player` = opponent) |
  | B2 | Opponent **Security effect** -DP (the security card's owner is the effect controller) | yes | blocked (same path) |
  | B3 | Opponent **static literal aura** "your opponent's Digimon get -N DP" (a materialized declarative `ChangeDp` with `source_player` = aura controller; `effect_context/action/modifiers.rs:126-147`) | yes | blocked |
  | B4 | Opponent **formula aura** with a negative value (`dp_modifier_fn`, summed by `live_declarative_formula_sum`, `game/mod.rs:3635-3730`) | yes | **NOT blocked**: `effective_dp` adds `dynamic_dp_aura_bonus` unfiltered (`dp.rs:83-84`) |
  | B5 | DP reduced to 0 or less → rules deletion (§17-1-3-1) | follows from B1-B4 | follows: no reduction means no deletion, except via B4 |
  | B6 | Opponent `<De-Digivolve N>` / `de_digivolve` step | BT26-029/085: yes. BT16-055: "isn't affected by <De-Digivolve> effects" | blocked for opponent sources |

- **Paths it must NOT block:**

  | # | path | engine today |
  |---|---|---|
  | N1 | Own effect -DP (`source_player == controller`) | applies (correct) |
  | N2 | Own De-Digivolve (cause `OwnEffect`) | applies. Correct for BT26-029/085 ("their effects"). **Disputed for BT16-055**: see defect 1 |
  | N3 | Deletion by an opponent's effect ("delete 1 of your opponent's Digimon") | not blocked (correct) |
  | N4 | Battle: DP comparison, deletion in battle, `<Piercing>` | not blocked. Battle doesn't reduce DP; only `ChangeDp` entries are filtered |
  | N5 | Opponent "delete Digimon with N DP or less" | not blocked (correct; it is a deletion) |
  | N6 | Bounce, return to deck, stack trash, suspension, "can't attack/block" | not blocked (outside the bundle) |
  | N7 | Positive DP from either side | not blocked |
  | N8 | Costs the controller pays, e.g. their own "-DP to this Digimon" as a cost | applies (own effect) |

- **Rule basis:**
  - §15-15-5: "isn't affected by effects" semantics. This protection is narrower than full immunity: it is category-scoped.
  - §16-12: `<De-Digivolve>`. §16-12-3: its trashing is mandatory.
  - §2-5-2 and §2-5-3: DP modification.
  - §15-1-3: prohibition precedence.
  - The "your opponent's effects" scoping comes from card text. The effect controller is determined per §15-2 (effect types, owner) and §4-11-2/-3.
- **Production users:**
  - **`BT26-029` Aegiochusmon: Holy** (decks 3-Jupitermon and 8-Jupitermon).
    - Printed: "[On Play] [When Digivolving] By trashing your top security card, until your opponent's turn ends, their effects can't reduce the DP of 1 of your Digimon, trash any of its stacked cards, or return them to hands or decks."
    - YAML (`cards/bt26/BT26-029.yaml`, OP/WD clause):
      1. `trash_top_security`
      2. `select_own_permanent { bind_as: protected, filter: { kind: digimon } }`
      3. `grant_narrow_opponent_effect_protection: { target: protected, expiry: end_of_opponents_turn }`
      4. `grant_stack_protection_from_opponent_effects`, with the same target and expiry
  - **`BT16-055` Namakemon.**
    - Printed: "If you have 3 or more security cards, until the end of your opponent's turn, 1 of your Digimon can't have its DP reduced by your opponent's effects and isn't affected by ＜De-Digivolve＞ effects."
    - YAML (`cards/bt16/BT16-055.yaml`):
      1. `select_own_permanent { bind_as: target, filter: { kind: digimon } }`
      2. `grant_narrow_opponent_effect_protection: { target: target, expiry: end_of_opponents_turn }`
  - Also `BT26-085` Giant Slayer, which uses `target: source`.
- **Existing near-miss tests:** none credits the verb.
  - `src/dsl_cards/step/modifiers.rs` is source.
  - `tests/cards_behavioral/bt19/bt19_089.rs` mentions the verb, but `cards_behavioral` is never scanned.
  - Prior art for the modifiers themselves:
    - `tests/track_c_immune_dp_minus.rs` is a top-level file, which is not scanned.
    - `tests/replacements/passive_modifier_migration.rs` installs `CannotBeDeDigivolved` directly, not via the verb.
- **Proposed behavioural test (for the plan):** one file with several `#[test]`s sharing fixtures.
  - Grant fixture:
    ```yaml
    card: TST-NARROW
    name: Narrow Protection Grant
    kind: digimon
    level: 4
    color: [black]
    cost: 4
    dp: 5000
    effects:
      - when: on_play
        process:
          - grant_narrow_opponent_effect_protection:
              target: source
              expiry: end_of_opponents_turn
    ```
  - DP-minus fixture (placed on P1 for the opponent rows, or on P0 for the own rows):
    ```yaml
    card: TST-DPMINUS
    name: DP Minus Source
    kind: tamer
    color: [purple]
    cost: 2
    effects:
      - when: on_play
        process:
          - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_is: "Narrow Protection Grant" }, prompt: "-3000" }
          - add_dp_modifier: { target: t, value: -3000, expiry: end_of_turn }
    ```
    `select_any_permanent` exists (`StepSpec::SelectAnyPermanent`). If its filter semantics differ, use `select_opponent_permanent` for the P1 copy and `select_own_permanent` for the P0 copy.
  - Analogous fixtures are needed for:
    - `TST-DEDIGI`: `de_digivolve: { target: t, amount: 1 }`.
    - `TST-KILL`: `delete_permanent: { target: t }`.
    - `TST-FORMULA-AURA`: an opponent aura that uses `dp_modifier_fn` with a negative value aimed at the opponent's Digimon. Copy the shape from an existing `dp_modifier_fn` aura card.
  - Setup:
    - P0 is the turn player.
    - Place TST-NARROW on P0's field as a stack [Lv3 source, TST-NARROW], so that De-Digivolve has something to trash.
    - Fire its OnPlay.
    - Assert that `runner.modifiers()` now has `ImmuneFromDPMinus` and `CannotBeDeDigivolved` on it.
  - Positive cases (blocked):
    - **B1:** fire TST-DPMINUS on P1 → `runner.effective_dp(h) == 5000`.
    - **B6:** fire TST-DEDIGI on P1 → `card_sources.len() == 2` (unchanged).
    - **B3**, if a literal opponent aura fixture is cheap to build → DP unchanged.
  - Negative cases (must NOT block):
    - **N1:** fire TST-DPMINUS on **P0** → DP 2000.
    - **N2:** fire TST-DEDIGI on **P0** → the top card is trashed (len 1). This pins the BT26 scoping.
    - **N3:** fire TST-KILL on P1 → the protected Digimon is in the trash.
    - **N4:** a 6000 DP P1 Digimon battles it (`runner.battle_digimon`) → it is deleted.
  - Expiry:
    - The protection holds through P1's turn: `pass_turn()`, then fire B1 → DP unchanged.
    - The protection ends with P1's turn: `pass_turn()` again → the modifiers are gone, and B1 → DP 2000.
  - Adjacent paths that must remain unaffected (both are outside this bundle):
    - An opponent `return_to_hand` still bounces TST-NARROW.
    - An opponent `trash_all_sources` still trashes its stacked cards.
  - Known-gap probe for B4, using the formula aura. Choose one of:
    - assert the *printed* expectation (DP unchanged) under `#[ignore = "B4: formula -DP auras bypass ImmuneFromDPMinus"]`; or
    - pin the current behaviour with a TODO.

    The plan decides which.
  - Selection surface: the verb surfaces none. In the BT16-055 shape, the preceding `select_own_permanent` must be mandatory (`is_optional == false`).
- **Suspected engine/DSL defects (read-only):**
  1. **BT16-055's De-Digivolve scope is over-narrowed.**
     - Printed: "... can't have its DP reduced by your opponent's effects and isn't affected by ＜De-Digivolve＞ effects". The De-Digivolve clause is unqualified.
     - DCGO `BT16_055.cs:65-70` installs `ImmuneFromDeDigivolveClass` with **no** opponent condition. Only `GainImmuneFromDPMinus` carries `IsOpponentEffect`.
     - The verb opponent-scopes both halves, so BT16-055 loses protection against its controller's own De-Digivolve.
     - BT26-029/085 *are* scoped to "their effects", so the verb is right for them. BT16-055 needs a variant.
  2. **Formula (`dp_modifier_fn`) -DP auras bypass `ImmuneFromDPMinus`** (`combat/dp.rs:83-84`). `live_declarative_formula_sum` only checks `is_unaffected_by` and the Progress shield (`game/mod.rs:3624-3633`).
  3. **Scoping can flip with control.** The opponent/own decision uses `entry.source_player != handle.player` at read time. If control of the permanent changes, the scoping silently flips. Edge case.

---

### `verb:grant_stack_protection_from_opponent_effects`

- **Lowering site(s):**
  - Parse: DSL `step.rs:441`. It reuses `GrantNarrowOpponentEffectProtectionArgs { target, expiry }`. Compiled form: `compiled.rs:2435`, `compile.rs:3875-3880`.
  - Step: ENG `dsl_cards/step/modifiers.rs:459-471`.
  - Engine: `effect_context/action/modifiers.rs:528-562`:
    ```rust
    if !self.can_affect_permanent(target) { return; }
    self.game.modifiers.add(target, ModifierEntry::simple(ModifierType::ImmuneFromStackTrashing, 0, expiry, self.player)
        .with_effect_immunity_filter(EffectImmunityFilter { source_kind: None, controller: EffectControllerFilter::OpponentOnly }));
    for modifier in [ModifierType::CannotBeDeDigivolved, ModifierType::CannotBeReturnedToHand, ModifierType::CannotBeReturnedToDeck] {
        self.game.modifiers.add(target, ModifierEntry::passive_replacement(modifier, expiry, self.player));
    }
    self.game.mark_until_condition_dirty();
    ```
  - Consult sites:
    - **`ImmuneFromStackTrashing`** is read by `Modifiers::blocks_effect_from` (`modifiers.rs:1438-1456`). There, OpponentOnly means "effect controller ≠ the permanent's controller". It is reached through:
      - `source_card_trash_blocked` (`effect_context/action/trash.rs:29-40`)
      - `stack_trash_blocked` (`trash.rs:52-58`)
    - **The three passive replacements** carry the cause filter `OpponentEffect` (`modifiers.rs:590-593`). They fire at `WhenWouldBeDeDigivolved`, `WhenWouldBeReturnedToHand` and `WhenWouldBeReturnedToDeck` (`replacement.rs:935-938`).
- **YAML shape:**
  ```yaml
  - grant_stack_protection_from_opponent_effects:
      target: protected
      expiry: end_of_opponents_turn
  ```
- **Semantics:**
  - The verb installs four protections on one permanent until `expiry`, all scoped to the opponent's effects:
    - stack-trash immunity
    - a cancel of De-Digivolve
    - a cancel of return-to-hand of **the permanent**
    - a cancel of return-to-deck of **the permanent**
  - "Stacked cards" means every card in a stack of 2 or more cards, top card included (§4-7-1/2).
  - A bounce moves the top card, which is a stacked card, to the hand. So blocking the whole-Digimon bounce follows the printed "return them to hands or decks".
  - No selection.
- **Paths the grant MUST block** (opponent effects). Printed BT26-029: "their effects can't ... trash any of its stacked cards, or return them to hands or decks".

  | # | path | consult site | engine today |
  |---|---|---|---|
  | S1 | `trash_all_sources` (all sources) | `trash.rs:559-562` (`self.player != target.player && stack_trash_blocked`) | blocked |
  | S2 | `trash_top_source` / `trash_top_stacked_sources` (pop the top) | `pop_top_source_for_trash`, `trash.rs:667-680` | blocked |
  | S3 | `select_opponent_sources` → `trash_selected_sources` / `trash_bottom_sources` / single `trash_card_source` | `trash.rs:504-510` (`source_card_trash_blocked`) | blocked (per card) |
  | S4 | `<De-Digivolve>` / `de_digivolve` | `effect_context/action/digivolve.rs:105-111` (`stack_trash_blocked`) + the `CannotBeDeDigivolved` passive | blocked (twice) |
  | S5 | Return the Digimon to **hand** (`return_to_hand`, `return_all_to_hand`) | the `WhenWouldBeReturnedToHand` passive cancel | blocked |
  | S6 | Return the Digimon to **deck** (`return_to_deck`, `return_all_to_deck`) | the `WhenWouldBeReturnedToDeck` passive cancel | blocked |
  | S7 | `return_top_stacked_to_deck` (zone_cards) | `dsl_cards/step/zone_cards.rs:345-363` reads `CannotBeReturnedToDeck` + cause | blocked |
  | S8 | **Return individual stacked cards to hand**: `return_selected_sources_to_hand` | `return_card_source_to_hand` → `stack_return_blocked`, which reads `ImmuneFromStackReturn` only (`effect_context/action/sources.rs:953-959`, `trash.rs:63-69`) | **NOT blocked**: the grant does not install `ImmuneFromStackReturn` |
  | S9 | **Return individual stacked cards to deck**: `return_selected_sources_to_deck`, `return_all_sources_to_deck` | `return_card_source_to_deck` → `ImmuneFromStackReturn` (`sources.rs:1036-1045`) | **NOT blocked** (same cause) |
  | S10 | DP reduction | n/a: that is `grant_narrow_opponent_effect_protection`'s half; BT26-029 installs both | see the previous section |

- **Paths it must NOT block:**

  | # | path | engine today |
  |---|---|---|
  | M1 | Own effects trashing or returning its sources. Own `trash_all_sources` skips the check via `self.player != target.player`, and `blocks_effect_from` is controller-scoped | applies (correct) |
  | M2 | Own costs, e.g. "by trashing 1 of this Digimon's digivolution cards" | applies |
  | M3 | **Deletion** by an opponent's effect, which trashes the stack (§4-7-8) | not blocked. Correct: §4-16-3 says "Trashing isn't considered deletion", and DCGO's `ImmuneStackTrashing` isn't consulted on deletion |
  | M4 | Deletion in battle | not blocked |
  | M5 | Own bounce / own return to deck | applies (cause `OwnEffect`) |
  | M6 | Opponent "place this Digimon as a source / into security / under a Tamer" (it leaves the field; this is not a trash or a return) | not blocked. The printed text is silent, so this is plausibly correct |
  | M7 | Suspension, DP, keyword removal | not blocked |

- **Rule basis:**
  - §4-7-1 and §4-7-2: stacked cards.
  - §4-7-8: the cards under a removed card are trashed at the same time.
  - §4-16-3: trashing is not deletion.
  - §16-12: De-Digivolve trashes the top stacked cards. The official BT26-085 Q&A says de-digivolve-style trashing counts.
  - §15-15-5: the category of "isn't affected".
  - §15-1-3: prohibition precedence.
- **Production users:**
  - **`BT26-029` Aegiochusmon: Holy** (decks 3-Jupitermon and 8-Jupitermon). This is the **only** user.
    - Printed: "[On Play] [When Digivolving] By trashing your top security card, until your opponent's turn ends, their effects can't reduce the DP of 1 of your Digimon, trash any of its stacked cards, or return them to hands or decks."
    - YAML (`cards/bt26/BT26-029.yaml`), after the narrow grant:
      ```yaml
      - grant_stack_protection_from_opponent_effects: { target: protected, expiry: end_of_opponents_turn }
      ```
  - Nearest relative: `BT26-085` Giant Slayer uses the sibling `grant_stack_trash_immunity_from_opponent_effects`, which covers trashing and De-Digivolve only.
- **Existing near-miss tests:** none credits the verb.
  - `src/dsl_cards/step/modifiers.rs` is source only.
  - Prior art installs the raw modifiers without the verb, so it credits nothing here. Two tests in `tests/dsl/stack_pair_and_protection.rs` (lines 265-293) install `ImmuneFromStackReturn` / `ImmuneFromStackTrashing` directly:
    - `stack_return_protection_blocks_only_opponent_returns`
    - `stack_trash_protection_is_controller_scoped_and_blocks_de_digivolve`
- **Proposed behavioural test (for the plan):**
  - Grant fixture:
    ```yaml
    card: TST-STACKGUARD
    name: Stack Guard Grant
    kind: digimon
    level: 5
    color: [yellow]
    cost: 6
    dp: 7000
    effects:
      - when: on_play
        process:
          - grant_stack_protection_from_opponent_effects:
              target: source
              expiry: end_of_opponents_turn
    ```
  - Attacker fixtures: one inline YAML per path. Each is a P1 Tamer whose `on_play` runs `select_opponent_permanent { bind_as: t, filter: { kind: digimon } }` and then applies the step. Reuse the same YAML on P0 for the "own" rows.
    - S1: `trash_all_sources: { target: t }`.
    - S2: `trash_top_source: { target: t }`.
    - S3: `select_opponent_sources: { target: t, min: 1, max: 1, bind_as: s }` + `trash_selected_sources: { source_refs: s }`.
    - S4: `de_digivolve: { target: t, amount: 1 }`.
    - S5: `return_to_hand: { target: t }`.
    - S6: `return_to_deck: { target: t, position: bottom }`.
    - S8: `select_opponent_sources` + `return_selected_sources_to_hand: { source_refs: s }`.
    - S9: `return_all_sources_to_deck: { target: t, position: bottom }`.
    - M3: `delete_permanent: { target: t }`.
  - Setup:
    - Build the P0 stack [Lv3 A, Lv4 B, TST-STACKGUARD] with `place_stack`, then fire the grant's OnPlay.
    - For each attacker row, use a fresh runner. Place the attacker on P1, fire OnPlay, and resolve any P1 pick (the selecting player is 1).
  - Positive (blocked) assertions:
    - S1-S4: P0's `card_sources.len() == 3`, and P0's trash is unchanged.
    - S5/S6: the Digimon is still in the battle area, and P0's hand and deck sizes are unchanged.
    - S8/S9: the **printed** expectation is that the sources stay, but today they move. Mark these rows `#[ignore = "S8/S9: grant lacks ImmuneFromStackReturn"]`, or pin the current behaviour with a TODO. The owner decides.
  - Negative (must NOT block) assertions:
    - M1: the same S1 and S3 fixtures, fired from **P0**, remove the sources.
    - M5: an own `return_to_hand` bounces the Digimon.
    - M3: an opponent `delete_permanent` deletes it, and its whole stack goes to the trash.
  - Expiry: after P1's turn ends (two `pass_turn()` calls from P0's turn), S1 from P1 succeeds again.
  - Selection surface: the verb surfaces none. The attacker fixtures' picks must be real `pending_selection`s for **P1**.
- **Suspected engine/DSL defects (read-only):**
  1. **Returning individual stacked cards is unprotected (S8/S9).**
     - The printed text covers "return them [its stacked cards] to hands or decks".
     - DCGO `BT26_029.cs:115` installs `CanNotBeReturnedToLibraryBySkillStaticEffect("Stacked cards may not be returned to hand or deck.")`.
     - Our grant installs `CannotBeReturnedToHand` / `ToDeck`, which are whole-Digimon replacements.
     - It does not install `ImmuneFromStackReturn`, the only modifier that `return_card_source_to_hand` / `_to_deck` consult.
  2. **`trash_all_sources` has a redundant own-player pre-check.**
     - It checks `self.player != target.player` before the protection (`trash.rs:560`). This is harmless here.
     - But it means an `Any`-scoped `ImmuneFromStackTrashing` never blocks the controller's own `trash_all_sources`, whereas `trash_card_source` would.
     - The inconsistency matters for other grants that use the `Any` scope.

---

### `verb:suspend_and_cancel_all_matching_leaves`

- **Lowering site(s):**
  - Parse: DSL `step.rs:186` (`SuspendAndCancelAllMatchingLeaves(SuspendAllPreventionArgs)`).
    - The args are `{ target: BindingRef, filter: PredicateSpec }` (`step.rs:1784-1790`). The doc comment on `filter` says "Subject qualification only; exclude activation-cost availability".
    - Validator: `validator.rs:1533-1536, 2192-2194`.
    - Compiled form: `compiled.rs:3141`, `compile.rs:3080-3083`.
  - Replacement-builder wiring: ENG `dsl_cards/lower_replacement.rs:112-117`.
    - A serde probe (`PaidPreventionProbe`, `lower_replacement.rs:3150-3205`) finds the step's `filter` anywhere in `process`.
    - It then calls `builder.prevention_all_matching().paid_prevention_filter(filter)`, which sets `Effect.paid_prevention_all = true` (`effect.rs:1469-1474`).
    - It also keys once-per-turn on the runtime (`with_opt_key`).
    - A replacement may contain only **one** such filter. The probe asserts this.
  - Step body: ENG `dsl_cards/step/replacement_outcome.rs:19-29`:
    ```rust
    CompiledStep::SuspendAndCancelAllMatchingLeaves { target, .. } => {
        if let Some(ResolvedBinding::Permanent(payer)) = resolve_binding_ref(target, ctx, bindings) {
            if ctx.can_affect_permanent(payer) {
                if let Some(card) = ctx.game.suspend_state_only(payer) {   // None if CannotSuspend or already suspended
                    ctx.game.enqueue_on_suspend(payer, card, true);
                    ctx.game.reevaluate_until_condition_modifiers();
                    set_outcome(ctx, ReplacementOutcome::Cancelled);
                }
            }
        }
        true
    }
    ```
    `suspend_state_only` is at `game/suspend.rs:73-136`.
  - Plural application (the "all matching" part) lives in ENG `replacement_deletion_batch.rs`:
    - **`activate_deletion_candidate`** (`:158-168`): when the candidate is `paid_prevention_all`, it skips the "which deletion does this process" subject prompt, even if several members match.
    - **`run_deletion_candidate_body`** (`:170-191`):
      - It snapshots `replacement_deletion_members`: every batch member that is still `!prevented && !terminal`.
      - It records `active_paid_all = (filter, members, source_card, controller, source_identity)`.
    - **`finish_deletion_candidate`** (`:219-243`): on `Cancelled`, it re-resolves each recorded member and evaluates `filter` per member, as a `Permanent` subject.
      - The `EffectReadContext` has `source_permanent` = the carrier.
      - Its replacement context is (cause relative to THAT member's controller, cause_player, member.player).
      - Every member that passes gets `prevented = true`.
    ```rust
    ReplacementOutcome::Cancelled=>{
        if let Some((filter,members,card,player,source_identity))=active_paid_all{
            let source_permanent = source_identity.and_then(|(owner, identity)| game.resolve_digivolve_host_identity(owner, identity));
            let valid:Vec<_>=members.iter().filter_map(|m|game.resolve_digivolve_host_identity(m.0,m.2))
                .filter(|h|{ let read=EffectReadContext::new(game,card,source_permanent,player)
                    .with_replacement_context(deletion_cause_for_controller(window.cause,window.cause_player,h.player),window.cause_player,Some(h.player));
                    eval_predicate(&filter,&read,PredicateSubject::Permanent(*h)) })
                .filter_map(|h|game.player(h.player).battle_area[h.index as usize].battle_identity.map(|id|(h.player,id))).collect();
            for m in &mut window.deletion_batch.as_mut().unwrap().members { if valid.contains(&(m.identity.0,m.identity.2)){m.prevented=true;} }
        } else { members[index].prevented=true; }
    }
    ```
  - Non-deletion departures use the same window machinery.
    - This covers `return_to_hand`, `return_to_deck` and `return_all_*`, via `start_departure_batch` (`replacement_departure_batch.rs:11-77`).
    - That function runs a `WhenWouldLeaveBattleArea` stage, then a route stage, `WhenWouldBeReturnedToHand` or `WhenWouldBeReturnedToDeck` (`:92-102`).
    - Both stages go through `advance_deletion_window` → `activate_deletion_candidate` → `finish_deletion_candidate`.
    - So the plural prevention applies to simultaneous returns as well.
- **YAML shape** (as in BT20-027, EX13-024, EX13-051 and BT22-019):
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    once_per_turn: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - kind: digimon
        - any_of: [{ in_text_contains: Dracomon }, { in_text_contains: Examon }]
        - none_of: [{ replacement_cause: battle }]
        - source_is_unsuspended: true
        - any_permanent: { of: you, is_source: true, can_change_orientation: true }
    process:
      - suspend_and_cancel_all_matching_leaves:
          target: source            # or a binding from a preceding optional select_own_permanent (EX13-024)
          filter:
            all_of:
              - replacement_subject_is_mine: true
              - kind: digimon
              - any_of: [{ in_text_contains: Dracomon }, { in_text_contains: Examon }]
              - none_of: [{ replacement_cause: battle }]
  ```
- **Semantics:**
  - **Payment.**
    - The step suspends exactly one permanent, `target`.
    - It cancels **only if an actual change from unsuspended to suspended happened**.
    - `CannotSuspend`, an already-suspended target, or "can't affect" all mean no change. Then there is no `Cancelled`, and the leave proceeds (§15-7-2).
  - **Scope of "all matching leaves".**
    - After a successful payment, every member of the **same simultaneous leave event** that is still pending is prevented if it satisfies `filter`.
    - The members are the Digimon in the current `delete_permanents_batch` call, or in the current simultaneous return batch (§3-1-3-3).
    - "Pending" means not already prevented, substituted or terminal.
    - `filter` is evaluated per member after the suspension. That is why production filters omit `source_is_unsuspended`, which would now be false.
    - `filter` is also evaluated with each member's own controller-relative cause. So `replacement_subject_is_mine` and `replacement_cause: opponent_effect` / `battle` are judged per member.
    - Members of a different, later event are not covered. Once-per-turn then blocks reuse within the turn.
    - A member that matches `active_when` but fails `filter` is **not** saved, even if it was the subject that triggered the replacement.
  - **Whether it is offered at all.**
    - That is decided by `active_when` (the replacement condition), evaluated per member, together with `optional`.
    - `predicate_reads_replacement_subject` must be true. Otherwise only the carrier itself is eligible (`lower_replacement.rs:138-141, 158`).
  - **No payability guard.**
    - `leading_cost_guard` (`lower_triggered.rs:1086`) does not recognise this step.
    - So §15-7-4 payability must be hand-authored in `active_when`, with `source_is_unsuspended` + `can_change_orientation`.
  - **Prompts.**
    - The optional replacement is an accept/pass prompt (`REPLACEMENT_ACCEPT` / `PASS`) for the replacement's controller.
    - With several matching members, a plural candidate does **not** get the "choose the impending deletion" subject prompt (`activate_deletion_candidate:161-164`).
    - A `select_own_permanent` before the step (as in EX13-024) is a real optional pick.
    - Declining, or failing to pay, leaves the outcome as `None`. The batch then continues, and every member still leaves.
  - **Outside a batch.**
    - If the leave is a single `try_replace` with no window, `set_outcome(Cancelled)` cancels just the current subject.
    - If the step is used in a body that is not a replacement, the outcome dangles in `dsl_replacement_outcome`. The validator does not restrict where the step may appear.
- **Rule basis:**
  - §15-8-5-1 and §15-8-5-2: immediate-type "when X would" effects interrupt right before the cause. Preventing the removal means the card isn't removed.
  - §15-8-5-4: once per cause.
  - §3-1-3-3: multiple cards leaving at the same time are one event. This is what bounds "all matching": "they" are the simultaneous subjects.
  - §15-7-1, §15-7-2 and §15-7-3: "by suspending ..." is an optional processing condition. No payment means no prevention, and there is no partial payment.
  - **§15-7-4 (Ver. 4.3, REVERSED):** "If executing an optional processing condition's contents isn't possible, then the player can't choose to execute them."
    - So an unpayable suspension must not be offered at all.
    - In 3.6 the text said the opposite. Engine comments citing 15-7-4 for "may decline" now mean §15-7-1 / §15-9-2-2.
    - See `docs/digimon-rules/changelog-3.6-to-4.3.md`, row §15-7-4 and section 4.1.
  - §15-7-5: the payment may be chosen even if nothing would be prevented.
  - **Note on the brief:** CR 4.3 §15-7 is "Optional Processing Conditions". The replacement-effect ("would") rules live in §15-8-5 (Immediate-Type Effects). Cite both.
- **Production users:**
  - **`EX13-024` Slayerdramon** (deck 6-Examon). This is the only top-10 user.
    - Printed: "[All Turns] [Once Per Turn] When any of your [Dracomon] or [Examon] text Digimon would leave the battle area, by suspending 1 of your such Digimon, they don't leave." The same text appears as an inherited effect.
    - YAML (`cards/ex13/EX13-024.yaml:131-176`):
      1. An optional pick:
         ```yaml
         select_own_permanent: { bind_as: suspend_cost, optional: true, continue_on_decline: true, filter: { kind: digimon, is_unsuspended: true, can_change_orientation: true, any_of: [...] } }
         ```
      2. Then:
         ```yaml
         if: { condition: { binding_exists: suspend_cost }, then: [ suspend_and_cancel_all_matching_leaves: { target: suspend_cost, filter: {...} } ], else: [ refund_opt: {} ] }
         ```
  - **`BT20-027`** (inherited).
    - Printed: "... other than in battle, by suspending this Digimon, they don't leave."
    - YAML (`cards/bt20/BT20-027.yaml:80-101`): the shape shown above, with `target: source`.
  - Also:
    - `EX13-051` Guardromon: `target: source`, and a filter with `other: true` and `has_keyword: Blocker`.
    - `BT22-019` Veemon: an `is_source: true` filter, i.e. self only.
- **Existing near-miss tests:** none credits the verb.
  - `src/dsl_cards/lower_replacement.rs` and `src/dsl_cards/step/replacement_outcome.rs` are source.
  - Prior art for the plural machinery with a different payer (a singular `delete_self` cost): `tests/paid_prevention_cardinality.rs`.
    - It is a top-level `tests/*.rs` file, not in a counted directory, and it does not use this verb.
    - But it shows the exact harness: `set_effect_source_player_for_test(Some(1))`, `delete_permanents_batch(vec![..], ReplacementCause::OpponentEffect)`, `REPLACEMENT_ACCEPT` and `build_action_mask`.
- **Proposed behavioural test (for the plan):** put it in `tests/replacements/` or `tests/dsl/`.
  - Fixture:
    ```yaml
    card: TST-SUSPALL
    name: Suspend All Guard
    kind: tamer
    color: [black]
    cost: 3
    effects:
      - kind: replacement
        trigger: when_would_leave_battle_area
        optional: true
        once_per_turn: true
        active_when:
          all_of:
            - replacement_subject_is_mine: true
            - kind: digimon
            - trait_has: Guarded
            - none_of: [{ replacement_cause: battle }]
            - source_is_unsuspended: true
            - any_permanent: { of: you, is_source: true, can_change_orientation: true }
        process:
          - suspend_and_cancel_all_matching_leaves:
              target: source
              filter:
                all_of:
                  - replacement_subject_is_mine: true
                  - kind: digimon
                  - trait_has: Guarded
                  - none_of: [{ replacement_cause: battle }]
    ```
  - Cards:
    - G1 and G2: Digimon with `traits: [Guarded]`.
    - X: a Digimon without the trait.
  - Setup:
    - P0 battle area = [G1, G2, X, TST-SUSPALL (unsuspended Tamer)].
    - Then:
      ```rust
      r.game.set_effect_source_player_for_test(Some(1));
      r.game.delete_permanents_batch(vec![G1, G2, X], ReplacementCause::OpponentEffect);
      ```
  - Positive case:
    - There is exactly one pending selection, for P0. It is optional, and `REPLACEMENT_ACCEPT` is legal in `build_action_mask`.
    - Accept it. Then:
      - G1 and G2 are still on the field.
      - X is in the trash.
      - TST-SUSPALL has `is_suspended == true`.
      - **There is no second prompt, and no "choose the impending deletion" subject prompt.**
      - An `OnSuspend` event fired once.
  - Negative cases:
    - (a) PASS → all three are deleted, and the Tamer stays unsuspended.
    - (b) Pre-suspend the Tamer → no prompt at all (the `active_when` gate). All three are deleted.
    - (c) Give the Tamer `CannotSuspend` → no prompt (the `can_change_orientation` gate). All three are deleted.
    - (d) Variant fixture: remove `source_is_unsuspended` and `can_change_orientation` from `active_when`, and pre-suspend the Tamer.
      - Today the prompt **is** offered, and accepting it saves nobody.
      - That is a §15-7-4 violation. Mark it as a known-gap probe.
    - (e) Battle: G1 loses a battle → no prompt (the `battle` cause is filtered out).
    - (f) Own effect: `delete_permanents_batch(.., OwnEffect)` with source player 0 → still offered, because the filter has no own-effect exclusion. This proves the cause is read per member.
    - (g) Once-per-turn: a second batch in the same turn (after an accept) → no prompt.
    - (h) Departure: P1 bounces G1 and X simultaneously (`return_all_to_hand` style) → after accept, G1 stays and X goes to the hand.
  - Selection surface:
    - The replacement prompt has `is_optional == true`, because printed "by suspending" is optional.
    - The payer is never auto-chosen in the EX13-024 shape. Add a variant that uses `target: chosen` after `select_own_permanent { optional: true }`.
    - In that variant, assert a real pick that lists only qualifying Digimon that are unsuspended and can change orientation.
  - Adjacent timing at which it must NOT fire: `when_would_be_de_digivolved`. An opponent `de_digivolve` on G1 must produce no prompt from TST-SUSPALL.
- **Suspected engine/DSL defects (read-only):**
  1. **No built-in §15-7-4 payability guard.**
     - `leading_cost_guard` does not recognise `SuspendAndCancelAllMatchingLeaves`, so the replacement is offered whenever `active_when` passes.
     - EX13-051 Guardromon gates only on `source_is_unsuspended`, not on `can_change_orientation`. So a `CannotSuspend` carrier is offered an unpayable prevention.
  2. **The plural scope exists only inside a batch window.**
     - Some simultaneous leaves do not go through `delete_permanents_batch` or `start_departure_batch`. Examples: "place all of your opponent's Digimon into security / under X", or "trash" a permanent.
     - Those hit one `try_replace` per subject. The first acceptance cancels one subject, and once-per-turn then blocks the rest.
     - Not verified; needs a probe.
  3. **A stricter `filter` can strand the triggering subject.**
     - If `filter` is stricter than `active_when`, the subject that triggered the replacement is not saved, even though the payment was made.
     - This is an authoring hazard: the two predicates must be kept in sync by hand, and cards duplicate them today.
