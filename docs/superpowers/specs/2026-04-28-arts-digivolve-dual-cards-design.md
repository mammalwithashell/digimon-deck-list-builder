# Arts Digivolve and DUAL Cards - Design Spec

**Date:** 2026-04-28
**Status:** Design spec. Not yet planned or implemented.
**Scope:** Rust engine, Rust DSL/card scripting surface, card-data ingestion, action masks, PyO3/Tauri-visible state.
**Primary rules sources:**
- Official rule-change page: `https://en.digimoncard.com/rule/revised/`
- Comprehensive Rules Manual Ver.4.0: `https://world.digimoncard.com/rule/pdf/general_rule.pdf?20260401=`
- Fandom DUAL Card page: `https://digimoncardgame.fandom.com/wiki/DUAL_Card`

## 1. Goal

Implement DUAL cards and Arts Digivolve in the Rust engine without approximations:

1. A DUAL card can be referenced as a Digimon card or an Option card in the correct contexts.
2. A DUAL card cannot be played as a Digimon because it has no Digimon play cost.
3. A DUAL card can be used as an Option card using its Option face: Option use cost, Option color requirements, and Option `[Main]` text.
4. A DUAL card can digivolve as a Digimon using its Digimon face: level, DP, Digimon colors, traits, digivolution requirements, and Digimon effects.
5. After a DUAL card is used as an Option card and its Option `[Main]` effect resolves, Arts Digivolve can optionally replace the normal pending Option trashing by stacking that in-flight DUAL card on one of the user's legal cards on the field without paying the digivolution cost.
6. Every player choice, including the choice to perform Arts Digivolve and the target card to digivolve, must surface through `PendingSelection` and the action mask.

## 2. Current State

The engine already has the right backbone for Option resolution:

- `Game::play_option_from_hand` and `Game::play_option_from_trash` route Options through `pending_option`, fire `OnUseOption`, fire `OptionMain`, then call `dispose_option`.
- `PendingOption` holds the in-flight Option card while it is not in a normal zone.
- `dispose_option` already branches by Option subtype: Standard, Delay, Link, Training.
- `digivolve_from_hand` and `digivolve_from_hand_onto_breeding` already perform standard hand-to-field digivolution, draw 1, and fire `WhenDigivolving` for battle-area digivolution.
- `CardData` currently has one `card_kind`, one `play_cost`, one color list, one level, one DP, one `evo_costs` list, and one set of text fields.

Known tracked gap:

- `docs/DCGO_KEYWORD_PARITY.md` explicitly marks Arts Digivolve as missing: no `Keyword::ArtsDigivolve` parser/enum path and no helper equivalent to DCGO's "digivolve executing Option card onto selected battle/breeding Digimon" primitive.

The missing part is not a generic Option pipeline. The missing part is a two-face card model and an Arts-specific dispose replacement.

## 3. Rules Summary

DUAL cards:

- A DUAL card is a single card that can be treated as either a Digimon card or an Option card.
- When treated as a Digimon card, only Digimon-face information is used.
- When treated as an Option card, only Option-face information is used.
- Effects that reference a broad "card" or "card with text" can reference all information printed on the DUAL card.
- When stacked on top of a card on the field, the DUAL card is treated as a Digimon.
- When a DUAL card is treated as a Digimon, it cannot also be referenced as an Option card.
- To use a DUAL card as an Option, the Option-face color requirements must be met. Digimon-face colors do not satisfy this requirement for the DUAL card itself.
- After its Option `[Main]` effect resolves, it would normally be trashed as an Option. Arts Digivolve may replace that trashing.

Arts Digivolve:

- Arts Digivolve is a rule on DUAL cards.
- It only applies when the DUAL card was used as an Option card.
- It does not apply when an effect directly activates the Option `[Main]` text without using the card as an Option.
- It is optional.
- If performed, one of the user's cards on the field digivolves into the in-flight DUAL card without paying the digivolution cost.
- Digivolution requirements must still be met.
- A digivolution bonus draw is still performed.
- After Arts placement, rule checks happen before triggered effects that occurred during the process are activated.
- The `[When Digivolving]` effect on the new DUAL Digimon and other effects triggered during the processing are considered simultaneous.

## 4. Non-Goals

- Implementing individual printed DUAL card effects. This spec delivers the substrate; card scripts land separately.
- Implementing every possible broad text-reference query up front. The minimum required for DUAL correctness is context-aware Digimon/Option queries plus an explicit all-text query for effects that ask for text on a card.
- Reworking DNA Digivolve, App Fusion, Link, Delay, or Training beyond what Arts needs.
- Fixing all rule-check residuals across the engine. Arts needs one explicit post-Arts rule-check hook, but a full rule-check engine can be broader follow-up work.
- Adding new Python legacy engine behavior. Rust is the target.

## 5. Design Principles

1. **Context is explicit.** DUAL cards are not "sometimes Digimon by accident." Every query asks for a face context: Digimon, Option, field-top Digimon, broad card text, or deck-building identity.
2. **No hidden choices.** The Arts yes/no choice and Arts target selection must be represented by legal action bits.
3. **Reuse pending-option state.** Arts is an alternative terminal path for an existing `PendingOption`, not a second Option resolver.
4. **No fake play cost.** DUAL cards must not get a synthetic Digimon play cost just to fit the existing `play_from_hand` path.
5. **Prefer small helpers over scattered conditionals.** Most engine code should call face-aware helpers rather than hand-checking `CardKind::Dual`.

## 6. Data Model

### 6.0 Live digimoncard.io API Shape

As of 2026-04-28, local `data/cards.json` does not contain DUAL cards. It has 4,085 cards, zero matches for `DUAL` or `Arts Digivolve`, and `card_kind` values only in the existing `0..3` range.

The live digimoncard.io API does contain DUAL cards. `type=Dual` returns DUAL rows even though the published API documentation still lists only `Digimon`, `Option`, `Tamer`, and `Digi-Egg` as type values.

Observed examples include:

- `ST23-09` Atratusmon
- `ST24-07` ShineGreymon
- `BT25-043` Habakirimon
- `BT25-057` Monarchlizamon
- `BT25-085` BeelStarmon
- `BT25-104` ShineGreymon: Burst Mode
- `EX12-018` Siriusmon

The API row is flattened rather than explicitly two-faced. Observed mapping:

| API field | Observed DUAL meaning | Engine/importer treatment |
|---|---|---|
| `type: "Dual"` | Identifies the card as DUAL | Map to `CardKind::Dual`. |
| `level`, `dp` | Digimon-face level and DP | Map to `dual.digimon.level` / `dual.digimon.dp` and compatibility top-level Digimon fields. |
| `color`, `color2` | Digimon-face colors in observed rows | Map to `dual.digimon.colors`. Do not assume these are Option-face colors. |
| `digi_type`, `digi_type2` | Digimon-face traits/archetype tags | Map to `dual.digimon.traits` and compatibility top-level traits. |
| `play_cost` | Appears to be Option use cost, not Digimon play cost | Map to `dual.option.use_cost`. Do not make the card playable as a Digimon. |
| `main_effect` | Digimon-face text | Map to `dual.digimon.effect_text`. |
| `source_effect` | Option-face text for DUAL rows | Map to `dual.option.effect_text`. This field is overloaded; for normal Digimon it remains inherited text. |
| `alt_effect` | Digimon-face digivolution requirements as text | Parse into `dual.digimon.evo_costs` where possible; preserve raw text for fallback/manual card scripting. |
| explicit Arts field | Not present | Infer Arts eligibility from DUAL rules/card type for current known DUAL cards, or add a card-data override if future data differentiates. |
| explicit Option-face colors | Not present | Parse from Option-face layout if a reliable source becomes available; otherwise use manual overrides per card. |

This shape is enough to bootstrap ingestion, but it is not enough to use the API response directly as the engine card model. The importer must normalize the flattened API row into the explicit `DualCardData` shape below. Any field that cannot be confidently derived, especially Option-face colors or structured digivolution requirements, must be represented as an importer gap or manual override rather than guessed silently.

### 6.1 CardKind

Add a new card kind:

```rust
pub enum CardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
    Dual,
}
```

`CardKind::Dual` means the card has both a Digimon face and an Option face. It does not mean it is simultaneously both in every context.

### 6.2 DualCardData

Extend `CardData` with an optional DUAL payload:

```rust
pub struct CardData {
    pub card_kind: CardKind,
    pub level: Option<u8>,
    pub dp: Option<i32>,
    pub play_cost: u16,
    pub colors: Vec<CardColor>,
    pub traits: Vec<String>,
    pub evo_costs: Vec<EvoCost>,
    pub effect_text: String,
    pub inherited_text: String,
    pub security_text: String,
    pub keywords: Vec<Keyword>,
    pub dual: Option<DualCardData>,
    // existing fields...
}

pub struct DualCardData {
    pub digimon: DualDigimonFace,
    pub option: DualOptionFace,
}

pub struct DualDigimonFace {
    pub level: u8,
    pub dp: i32,
    pub colors: Vec<CardColor>,
    pub traits: Vec<String>,
    pub evo_costs: Vec<EvoCost>,
    pub effect_text: String,
    pub inherited_text: String,
    pub keywords: Vec<Keyword>,
}

pub struct DualOptionFace {
    pub use_cost: u16,
    pub colors: Vec<CardColor>,
    pub effect_text: String,
    pub security_text: String,
    pub keywords: Vec<Keyword>,
}
```

For non-DUAL cards, existing fields remain the source of truth. For DUAL cards:

- Existing top-level fields should mirror the Digimon face for backward-compatible tensor/card-display code that expects level, DP, traits, and evo costs on a field Digimon.
- Option-specific behavior must read `dual.option`, not the top-level fields.
- Generic card text search should concatenate both faces.
- `play_cost` for `CardKind::Dual` must not make the card playable as a Digimon. The DUAL Digimon face has no play cost. The Option face has `use_cost`.
- Imported digimoncard.io DUAL rows must map their API `play_cost` into `dual.option.use_cost`, not into a Digimon play path.
- Imported digimoncard.io DUAL rows must map API `source_effect` into Option-face text, not inherited Digimon text.

### 6.3 Face-Aware Queries

Add helpers on `CardData` or `CardSource`:

```rust
fn is_digimon_card_for_search(&self) -> bool;
fn is_option_card_for_search(&self) -> bool;
fn field_kind(&self, data: &[CardData]) -> CardKind;
fn digimon_level(&self, data: &[CardData]) -> Option<u8>;
fn digimon_dp(&self, data: &[CardData]) -> Option<i32>;
fn digimon_colors(&self, data: &[CardData]) -> &[CardColor];
fn option_colors(&self, data: &[CardData]) -> &[CardColor];
fn option_use_cost(&self, data: &[CardData]) -> Option<u16>;
fn digivolution_costs(&self, data: &[CardData]) -> &[EvoCost];
fn text_for_search_all_faces(&self, data: &[CardData]) -> String;
```

Use these helpers at action mask, digivolution, Option color checks, search predicates, and DSL predicate evaluation sites.

## 7. Action Mask and Decoder Semantics

### 7.1 Main-Phase Play Bits

The play-from-hand range `0..29` currently handles Digimon, Tamers, and Options. With DUAL:

- `CardKind::Option`: play bit is legal if Option use cost is affordable and color requirements pass.
- `CardKind::Dual`: play bit means "use as Option," not "play as Digimon."
- `CardKind::Dual` play bit is legal if the DUAL Option face use cost is affordable and Option-face color requirements pass.
- `CardKind::Dual` must never route to `play_from_hand`.

Decoder branch:

```rust
match card_kind {
    CardKind::Option => play_option_from_hand(...),
    CardKind::Dual => play_dual_as_option_from_hand(...),
    CardKind::Digimon | CardKind::Tamer => play_from_hand(...),
    _ => {}
}
```

`play_dual_as_option_from_hand` can delegate to the same Option core with a source flag that identifies this as true Option use. This flag is required so Arts can distinguish "used as an Option" from "OptionMain activated directly by an effect."

### 7.2 Digivolve Bits

The digivolve range `400..999` should consider:

- `CardKind::Digimon`
- `CardKind::Dual`

For DUAL, eligibility uses the Digimon face:

- Digimon-face level and evo costs.
- Digimon-face colors on the evolving card where relevant.
- Base card's field identity.

For a normal hand-to-field digivolve, DUAL digivolution pays the printed Digimon-face digivolution cost and uses existing digivolve behavior.

### 7.3 Hand Main Bits

Direct `[Hand][Main]` activation on a DUAL card must not grant Arts Digivolve unless the activation is actually "use this card as an Option." The condition is:

- Action `0..29` routed through Option use: Arts allowed after `OptionMain`.
- Action `30..59` routed through `activate_hand_main`: Arts not allowed unless a future explicit effect says to use the card as an Option through the Option pipeline.

## 8. Option Pipeline Changes

### 8.1 PendingOption Extension

Extend `PendingOption`:

```rust
pub struct PendingOption {
    pub owner: PlayerId,
    pub card: CardSource,
    pub resolution_phase: OptionResolutionPhase,
    pub source_kind: OptionUseSource,
}

pub enum OptionUseSource {
    UsedFromHand,
    UsedFromTrash,
    UsedFromSecurity,
    DirectMainActivation,
}
```

Arts is legal only when:

- `card.card_kind == CardKind::Dual`
- DUAL Option face has `Keyword::ArtsDigivolve`
- `source_kind` is `UsedFromHand`, `UsedFromTrash`, or `UsedFromSecurity` if that path truly used the card as an Option
- `source_kind` is not `DirectMainActivation`

If the engine later supports "use an Option from reveal" or "use an Option from deck," those should be represented as Option-use sources and can be Arts-eligible if the card was truly used as an Option.

### 8.2 OptionSubtype

Add Arts as a post-main disposal capability, not a normal subtype that parks on the field:

```rust
enum OptionSubtype {
    Standard,
    Delay(DelayTrigger),
    Link,
    Training,
}

fn has_arts_digivolve(effects: &[Effect], card_data: &CardData) -> bool;
```

Arts is checked before the Standard dispose path. Delay, Link, and Training should not combine with Arts unless printed card rules introduce such a card. For v1, if a DUAL card has Arts and also a persistent Option subtype flag, the engine rejects the card-data pack at validation time because the printed DUAL rules describe Arts replacing normal trashing after Option use.

### 8.3 New Resolution Phase

Add a phase:

```rust
pub enum OptionResolutionPhase {
    LinkSelectHost,
    MainEffectDrain,
    ArtsDecision,
    ArtsSelectTarget,
    Disposing,
    Done,
}
```

Flow:

1. Option use installs `PendingOption { source_kind: UsedFromHand, resolution_phase: MainEffectDrain }`.
2. `OnUseOption` and `OptionMain` resolve.
3. If a selection was parked by `OptionMain`, the existing resume hook returns to step 2 after the selection resolves.
4. If Arts is legal and at least one Arts target exists, install an optional Arts decision/target prompt.
5. If the player declines Arts, continue to `dispose_option`.
6. If the player accepts Arts, stack the pending card onto the selected target, draw 1, run post-Arts rule check, enqueue simultaneous triggers, clear `pending_option`, and check turn end.
7. If no Arts target exists, skip the Arts prompt and continue to `dispose_option`.

The prompt can be implemented as a single optional target prompt rather than a separate yes/no prompt:

- Valid target action IDs represent "perform Arts onto this card."
- PASS means "decline Arts and trash normally."

This is simpler for RL and still fully surfaces the choice.

## 9. Arts Target Selection

### 9.1 Target Scope

Official text says "one of your cards on the field." For v1:

- Battle-area Digimon are eligible if the DUAL Digimon face can legally digivolve onto them.
- Breeding-area Digimon are eligible if the DUAL Digimon face can legally digivolve onto them.
- Tamer-as-Digimon and other "as if" alternate digivolution requirements are only eligible if existing engine helpers already support them. This spec does not add new alternate requirement parsing.

Target filters:

- Target must be controlled by the Option user.
- Target must not have `CannotDigivolve`.
- Target must satisfy a printed DUAL Digimon-face digivolution requirement.
- Target may be in battle area or breeding area.

### 9.2 Action Encoding

Use existing action conventions where possible:

- Battle-area target: `encode_attack(0, field_index)` in the `100..399` range, matching own-field selection helpers.
- Breeding target: add a stable selection action ID. Recommended: use `99` for own breeding permanent in selection phases, matching the existing selection convention in `docs/ACTION_SPEC.md`.

The mask should emit only `pending_selection.valid_action_ids` during Arts selection, plus PASS because the selection is optional.

### 9.3 Selection Kind

Add a selection helper rather than hand-building prompts in `game_actions.rs`:

```rust
EffectContext::select_own_field_or_breeding(...)
```

or a game-action-local helper:

```rust
Game::install_arts_digivolve_selection(owner, source_card, candidates)
```

The local helper is acceptable because Arts is rule processing, not a card-script effect body. It should still produce a normal `PendingSelection`.

## 10. Arts Execution Primitive

Add a dedicated primitive:

```rust
fn arts_digivolve_pending_option_onto(&mut self, target: ArtsTarget) -> bool;

enum ArtsTarget {
    Battle(PermanentHandle),
    Breeding(PlayerId),
}
```

Behavior:

1. Take `PendingOption`.
2. Re-validate that the pending card is a DUAL card with Arts and that `source_kind` allows Arts.
3. Re-validate that the target still exists and still satisfies digivolution requirements.
4. Move the pending card onto the target's digivolution stack without paying memory.
5. Draw 1.
6. Treat the DUAL card as a Digimon on top of the stack.
7. If target is battle area, enqueue `WhenDigivolving` for that permanent.
8. If target is battle area, enqueue global `OnDigivolve` observers after the self timing, following the existing `digivolve_from_hand` ordering unless the simultaneous-trigger refactor described below lands first.
9. If target is breeding area, follow the engine's existing breeding digivolution rule: no `WhenDigivolving` effects from breeding unless a later rules audit changes that behavior.
10. Run `check_turn_end` after all post-Arts processing completes.

The primitive must not call `pay_memory`.

## 11. Trigger Timing and Rule Checks

Official Arts timing says:

1. Option `[Main]` resolves.
2. Player chooses whether to Arts.
3. If Arts happens, the card is placed for digivolution and draw 1 happens.
4. A rule check occurs.
5. Effects triggered during the processing are considered simultaneous.

The current engine tends to enqueue and drain `WhenDigivolving` immediately after a digivolve. For Arts v1:

- Add a small `run_rule_check_after_arts` hook before draining Arts-triggered effects.
- The hook must at least delete battle-area Digimon with 0 DP and trash illegal field objects that are already supported by the engine.
- If the current engine lacks a general rule-check implementation, implement the minimum Arts-required rule check with tests and document the broader rule-check engine as follow-up.

For simultaneous trigger precision:

- Preferred: add a local event batch for Arts so `[When Digivolving]` and rule-check-generated `[On Deletion]` triggers are enqueued before the drainer asks for ordering.
- Acceptable v1 if the engine lacks trigger-batch support: enqueue `WhenDigivolving`, run rule check, enqueue deletion triggers, then drain in a way that still lets the turn player order simultaneous effects. If this cannot be achieved with the current queue, the implementation must add a queue-batch helper rather than draining early.

No implementation should silently auto-order simultaneous Arts triggers.

## 12. DSL and Card Scripting

### 12.1 Keyword Parsing

Add:

```rust
Keyword::ArtsDigivolve
```

Update `parse_printed_keywords` to recognize `Arts Digivolve`. If printed DUAL cards write the keyword outside the existing full-width keyword brackets, the data ingestion step must inject it into the Option face keyword list rather than relying only on text scanning.

### 12.2 Effect Builder

Add a builder marker only if needed:

```rust
EffectBuilder::arts_digivolve()
```

The preferred design is data-driven: Arts is a card keyword/rule, not an effect body. The builder marker is only useful for synthetic test cards or hand-authored raw Rust cards that do not go through DUAL metadata.

### 12.3 YAML DSL

Extend the card metadata schema to allow:

```yaml
kind: dual
dual:
  digimon:
    level: 6
    dp: 12000
    colors: [Red, Yellow]
    traits: [...]
    digivolve:
      - from:
          level_eq: 5
          color_is: Red
        cost: 4
    effects:
      - timing: when_digivolving
        ...
  option:
    use_cost: 5
    colors: [Purple]
    main:
      - ...
    keywords:
      - ArtsDigivolve
```

The DSL compiler should lower:

- Digimon face effects to normal Digimon timings.
- Option face main text to `EffectTiming::OptionMain`.
- Arts keyword to card metadata, not to an ordinary `OptionMain` step.

## 13. Search and Predicate Semantics

Update predicate evaluation in Rust engine and DSL:

- `kind: Digimon` matches DUAL cards when the query is searching cards in a non-field zone or when the DUAL is on top of a field stack as a Digimon.
- `kind: Option` matches DUAL cards when the query is searching cards in a non-field zone or when the DUAL is being used as an Option.
- A DUAL card stacked on the field as a Digimon does not match `kind: Option`.
- `has_text` and "card with X in its text" style predicates use all DUAL printed text unless the effect explicitly asks for Digimon text or Option text.
- Color predicates use the active face context. Generic card-search color predicates over a hidden/public zone can match either face only when the card text says "card" rather than "Digimon card" or "Option card."

Add a `PredicateContext` enum:

```rust
pub enum PredicateContext {
    CardSearchAny,
    DigimonCardSearch,
    OptionCardSearch,
    FieldDigimon,
    OptionUse,
    DigivolutionRequirement,
}
```

Long term, predicate evaluation should require this context. Short term, add explicit helper functions for the call sites touched by DUAL.

## 14. Serialization and UI/FFI Surface

Expose DUAL-aware data through existing state views:

- `CardDataView.cardKind` can be `"Dual"`.
- Include `dualDigimon` and `dualOption` payloads for clients that need to render both faces.
- During `pending_option`, expose `pendingOption.cardKind = "Dual"` and `pendingOption.canArtsDigivolve`.
- During Arts selection, expose prompt kind and legal action IDs through existing `pendingSelection`.
- Once Arts resolves, the top card of the resulting stack serializes as a Digimon. It should not also appear as an Option permanent.

No frontend rule decisions are allowed. UI only renders legal actions from the engine.

## 15. Tests

### 15.1 Data and Parsing

- Parse a synthetic DUAL card from fixture JSON with distinct Digimon and Option colors.
- `CardKind::Dual` round-trips through serialization.
- `parse_printed_keywords` recognizes Arts Digivolve.
- DUAL top-level compatibility fields mirror the Digimon face.
- Option face use cost and colors do not overwrite Digimon face data.

### 15.2 Action Mask

- A DUAL in hand emits play bit `0..29` when the Option face color requirement is met.
- A DUAL in hand does not emit a normal Digimon play path.
- A DUAL in hand emits digivolve bits `400..999` when its Digimon face can digivolve onto a field Digimon.
- Option color matching for DUAL uses Option face colors, not Digimon face colors.
- Digivolution matching for DUAL uses Digimon face evo requirements, not Option face colors.

### 15.3 Option Use

- Using a DUAL as an Option pays the Option face use cost.
- Using a DUAL as an Option fires Option face `OptionMain`.
- Direct `[Hand][Main]` activation of the same card's Option text does not enable Arts.
- If Arts is unavailable because no legal target exists, the card trashes normally.

### 15.4 Arts Decline

- After `OptionMain`, an optional Arts selection is installed when targets exist.
- PASS declines Arts.
- Declining Arts routes to existing Standard Option trash disposal.
- Existing `WhenWouldBeTrashed` replacement behavior still applies after decline.

### 15.5 Arts Accept

- Selecting a legal battle-area target stacks the pending DUAL card on that target.
- No memory is paid for the digivolution.
- The player draws 1.
- `WhenDigivolving` fires for the new DUAL Digimon.
- Global `OnDigivolve` observers fire.
- The pending Option is cleared.
- The card is not in trash.

### 15.6 Breeding Target

- A legal breeding-area target appears in Arts selection.
- Selecting the breeding target stacks the DUAL card onto breeding.
- The player draws 1.
- Breeding-area `WhenDigivolving` behavior follows the current engine rule and is covered explicitly.

### 15.7 Rule Check and Trigger Ordering

- If Arts causes a Digimon with 0 DP to exist at rule check, it is deleted before pending triggered effects activate.
- `[When Digivolving]` and `[On Deletion]` effects triggered during Arts processing are ordered through the normal simultaneous-trigger selection path when more than one order is legal.
- No test relies on fixed hidden ordering.

### 15.8 Predicate Semantics

- Searching for a Digimon card can find a DUAL in hand using its Digimon face.
- Searching for an Option card can find a DUAL in hand using its Option face.
- A DUAL stacked on field as a Digimon does not satisfy an Option-card-on-field predicate.
- Generic "card with text" search can find text from either face.

## 16. Rollout Plan

### Phase A - Data Model and Helpers

- Add `CardKind::Dual`.
- Add `DualCardData` and deserialization support.
- Add face-aware helper methods.
- Add synthetic DUAL fixture support in tests.
- Keep all existing non-DUAL tests passing.

### Phase B - Mask and Normal Use Paths

- Update play and digivolve action masks for DUAL.
- Route DUAL play bits through Option use.
- Route DUAL digivolve bits through standard digivolution.
- Add Option-face color and use-cost handling.

### Phase C - Arts Keyword and Selection

- Add `Keyword::ArtsDigivolve`.
- Add Arts eligibility checks after `OptionMain`.
- Add optional Arts target selection with PASS decline.
- Add battle-area target support.

### Phase D - Arts Execution

- Add `arts_digivolve_pending_option_onto`.
- Stack the in-flight DUAL card without paying memory.
- Draw 1.
- Fire `WhenDigivolving` and `OnDigivolve`.
- Clear pending state and check turn end.

### Phase E - Breeding and Rule Check Precision

- Add breeding target action support.
- Add post-Arts rule-check hook.
- Ensure simultaneous trigger ordering is visible.

### Phase F - DSL and Real Card Ingestion

- Extend DSL metadata schema for `kind: dual`.
- Lower Option face and Digimon face effects to correct timings.
- Update card ingestion for real DUAL cards once source data is available.
- Implement first real DUAL card as a behavioral test slice.

## 17. Compatibility and Migration

- Existing non-DUAL cards should continue using the current `CardData` fields.
- Existing Option, Delay, Link, and Training tests should not change except for any helper signatures that need predicate context.
- Existing `CardKind` numeric conversion must be versioned carefully because Python, PyO3, Tauri, and tensor code may assume specific discriminants.
- Observation tensor changes must be documented in `docs/TENSOR_SPEC.md` before adding a new encoded card kind. If the tensor currently has no card-kind slot for DUAL, encode DUAL in a backward-compatible way only after updating the tensor spec and frontend constants.
- Action IDs do not need a new global range. Arts uses pending-selection legal IDs.

## 18. Risks

### Risk: DUAL Data Source Shape

`data/cards.json` may not currently expose both faces. The implementation should start with synthetic fixtures and then update ingestion once real DUAL card data is available.

Mitigation: make the Rust deserializer accept an explicit `dual` object and keep legacy card shapes unchanged.

### Risk: Predicate Drift

DUAL touches many "is this a Digimon or Option" helpers. Scattered conditionals will produce bugs.

Mitigation: add face-aware helper methods and migrate call sites deliberately, starting with action mask, digivolution, Option use, and DSL predicates.

### Risk: Trigger Ordering

Official Arts timing requires rule checks before trigger activation, with simultaneous ordering. The existing immediate drain behavior may not be precise enough.

Mitigation: add tests first. If the queue cannot represent the timing, add a batch enqueue helper rather than relying on hidden order.

### Risk: Tensor Contract

Adding `CardKind::Dual` can affect RL observations.

Mitigation: update `docs/TENSOR_SPEC.md`, Rust constants, PyO3 views, and frontend constants in the same implementation phase if card kind is encoded in tensors.

## 19. Acceptance Criteria

The feature is complete when:

- Synthetic DUAL cards can be used as Options and digivolved as Digimon.
- Arts Digivolve can be declined or accepted through action-mask-visible choices.
- Arts acceptance stacks the pending DUAL card, draws 1, pays no memory, and fires digivolution triggers.
- Arts decline uses the normal Option trash/replacement path.
- DUAL face queries are correct for Option color, Digimon digivolution, search predicates, and field identity.
- No hidden auto-selection or UI-only decision exists.
- Rust engine tests cover data parsing, masks, Option use, Arts decline, Arts accept, breeding target behavior, and trigger/rule-check ordering.
