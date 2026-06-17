## Context

The action bar labels activatable effects with `Effect {source}:{effectIdx}` — built locally in `ActionBar.tsx` from a `canActivateEffect: Map<sourceSlot, Set<effectIdx>>` that `useActionMask.ts` derives by scanning the raw action mask. That re-derivation is a second, independently-maintained model of the engine's action layout, and it has drifted:

- The frontend treats `1000–1999` as one "effects" range and decodes `source = (id-1000)/10`. The engine actually packs **field [Main]** in `1000–1149`, **trash [Main]** in `1150–1194`. Trash actions therefore decode to battle slots `15–19`, which don't exist.
- The frontend treats `30–59` as "trash from hand". The engine uses `30–59` for **hand [Main]** activations (phase-disambiguated; in selection phases the same ids mean reveal/security picks). A `MainFromHand` effect lands in `canTrashFromHand` and is never offered as an effect.
- Even where the range is right (field `1000–1149`), the label is the slot plus the internal `FIELD_EFFECT_SLOT_FOR_MAIN = 2` constant — meaningless to a human — and the source card's name (already in the permanent DTO) is unused.

The engine side is correct and tested: `mask_and_tensor` (170 passing) verifies all three ranges emit with condition-gating, OPT, inherited-only-when-under, and per-slot independence. The engine also already has `legal_decoded_actions(game, player) -> Vec<ActionExplanation>` (`action/explain.rs`), which decodes each legal action id correctly and populates `card_id` / `card_name` per action — but it is exposed only on the debug/MCP `LiveGame` surface, not to the production Tauri/REST clients, and it does not read the effect's own `name`.

`Effect` already carries `pub name: String` (`effect.rs`), set by the builder `.name(...)`. Many DSL lowerings already name their effects (`lower_aura`, `lower_grant_keyword`, `lower_cost_reduction`, `lower_flood_gate`, `lower_replacement`, `lower_partition`); the activatable [Main] paths (notably `digi_burst`, `<Training>`, delayed-Option bodies) are the gaps. `<Digi-Burst N>` is a real keyword (`Keyword::DigiBurst(u8)`).

A separate, already-authored change `add-per-card-command-panel` proposes a DCGO-style click-the-card contextual menu; it plans to derive labels from `useActionMask` and fall back to generic `[Main] Effect 1` labels "until the engine's effect-text serialization lands." This change is that substrate.

## Goals / Non-Goals

**Goals:**
- One source of truth for activatable-effect labels: the engine decoder, consumed by the action bar instead of a parallel frontend re-derivation.
- Surface **every** engine-emitted activatable category — field [Main], Digiburst, breeding `<Training>`, trash [Main], hand [Main], delayed-Option [Main] — whenever legal.
- Label by source card + effect name (`"{card}: {effect}"`), slot only on duplicate card names, with main-effect-text tooltip.
- Effect-name in v1: `explain_action` resolves and returns the matched effect's `name`.

**Non-Goals:**
- No contextual per-card command panel (that is `add-per-card-command-panel`; it can consume this output).
- No change to action ids, `ACTION_SPACE_SIZE`, the mask, or legality logic — so no action-space version bump, and trained models / recordings are untouched.
- No new card-effect behavior; the DSL work is naming only.
- No redesign of the non-effect action-bar buttons (Pass, etc.) or the selection-phase UI.

## Decisions

### D1 — Render from the engine decoder, not from re-derived mask ranges

The action bar's activatable-effect entries are built from `legal_decoded_actions`, exposed over Tauri and REST. Rationale: the decoder already maps each action id to the right zone + card, so the trash/hand/training mis-decode class disappears by construction rather than by patching three more range branches that can drift again. `useActionMask`'s other capability maps (play, digivolve, attack, DNA) are unchanged for now — only activatable effects move to the decoder.

*Alternative considered — patch `useActionMask` ranges in place* (split `1000–1149` vs `1150–1194`, add a phase-aware hand-main branch, look up names from state). Rejected as the primary path: it keeps two sources of truth (the exact drift that caused this), and trash-card names are not even on the state wire, so it needs new DTO fields anyway. The decoder already carries `card_name` for trash.

### D2 — `effect_name` lives on the decoded action; resolved by mirroring first-match-wins

Add `effect_name: Option<String>` to `ActionExplanation` (and the `DecodedAction` exposed by `LiveGame`). For field/hand/trash [Main] ids, `explain_action` re-runs the same "first eligible effect of this timing on this carrier/card" walk the mask builder uses, then reads that effect's `name`. Rationale: the mask builder is first-match-wins, so exactly one effect is surfaced per slot — the decoder must select the same one to label it correctly. Keeping the resolution in `explain.rs` (next to the mask logic it mirrors) avoids spreading the selection rule across layers.

*Alternative — store the chosen effect index in the mask output and have explain read it back.* Rejected: the mask is a flat `Vec<f32>`; threading side-channel indices through it is heavier than re-resolving, and re-resolving keeps the decoder self-contained (matching the existing "decoder adds no legality logic of its own" contract).

### D3 — Label format and duplicate-name disambiguation are a frontend concern

The engine returns the parts (`card_name`, `effect_name`, `source_zone`, `source_index`); the frontend composes `"{card}: {effect}"`, drops the slot for unique names, and appends `(slot N)` only when two surfaced entries collide on card name. Rationale: "duplicate" is defined over the *currently surfaced set*, which only the rendering layer knows; the tooltip (main effect text) also already lives in the frontend's permanent/state DTO, matched by zone+index. Keeping composition in the frontend avoids the engine needing to know what else is on screen.

### D4 — Fill effect names on the activatable lowering paths, with card-name fallback

Audit the lowerings that emit [Main]-timed activatable effects and set `.name`: `digi_burst` → `"Digiburst"` (optionally `"Digiburst {N}"`), `<Training>`, and delayed-Option `[Main]` bodies. Where an effect still has no name, the label degrades gracefully to card-name-only (D3). Rationale: names should describe the *printed ability*, not the lowering internals; doing this on the lowering keeps every card of a kind consistent without per-card authoring.

### D5 — Two transport surfaces, both engine-only

Expose the decoder via (a) a Tauri command in `engine_commands.rs` returning the decoded list alongside the existing state/mask response, and (b) a hosted-API REST route on an engine-only router (no DB/auth imports, per the service-boundary rules). Rationale: desktop and browser both need it; both already round-trip state+mask, so the decoded list rides the same request shape.

## Risks / Trade-offs

- **Per-call decode cost** → `legal_decoded_actions` calls `explain_action` for every set bit each state update. Mask widths are small (low hundreds of legal actions at most) and decoding is cheap; acceptable. If it ever shows up, the call can be memoized per (state-hash, player).
- **Two label sources during transition** (action bar on the decoder, other buttons still on `useActionMask`) → entries could in principle disagree. Mitigation: scope the decoder strictly to activatable effects in v1; leave play/digivolve/attack on their existing maps until a later pass optionally migrates them.
- **DSL name audit misses a path** → an activatable effect ships with an empty name. Mitigation: card-name fallback (D3/D4) keeps it usable, and a lowering-name assertion test (e.g. `digi_burst` → "Digiburst") guards the named cases.
- **`effect_name` first-match-wins must match the mask exactly** → if the two walks diverge, the label names the wrong effect. Mitigation: factor the "first eligible [Main] effect for (carrier, timing)" selection into one shared helper used by both the mask builder and `explain_action`, and unit-test that they agree.
- **Relationship with `add-per-card-command-panel`** → both touch the action surface. Mitigation: this change deliberately makes no contextual-UI changes and produces the named/decoded list that the panel's design already anticipates consuming; ordering is independent.
- **Hand [Main] id reuse across phases** (`30–59` also encodes reveal/security selections) → the decoder must label by *current phase*. Mitigation: `explain_action` already decodes per current phase; the action bar only renders activatable effects during the Main phase.

## Open Questions

- Should `digi_burst` name carry the cost count (`"Digiburst 2"`) or stay generic (`"Digiburst"`)? Leaning generic for v1 (the printed keyword name), revisit if players want the cost inline.
- Do we migrate play/digivolve/attack labels to the decoder in this change too, or leave that to a follow-up once the action bar consumes the decoded list? Default: leave them; this change stays scoped to activatable effects.
