## Context

`standard_lite_v2` is the current serious-training observation profile. It encodes public board state, own hand, known zones, pending choices, and player zone counts, but it does not encode the original list the pilot is playing. That means a policy can see a card once it reaches hand, trash, revealed zones, or public board state, but it cannot condition long-range planning on deck construction.

The Rust `HeadlessRunner` already retains submitted `deck1_ids` and `deck2_ids` for recording, while the observation writer receives only `Game`, observer player ID, and the card registry. The new profile needs immutable original-deck composition available at observation time without relying on the runner as a side channel.

The change must preserve the no-approximations policy for action legality: legal choices continue to come from the action mask and pending-selection contracts. Decklist features are observation context only.

## Goals / Non-Goals

**Goals:**

- Add an opt-in `standard_lite_deck_v2` observation profile derived from `standard_lite_v2`.
- Encode the observing player's original submitted decklist as stable unique-card rows.
- Keep the profile fair-information by exposing only own original composition and no hidden order.
- Export complete layout metadata so Python feature extraction, model metadata, and compatibility checks remain layout-driven.
- Preserve all action IDs and action-mask semantics.

**Non-Goals:**

- Do not change `standard_lite_v2` tensor size or default behavior.
- Do not expose opponent decklists in this profile.
- Do not encode current shuffled deck order, face-down security identities, or topdeck information.
- Do not add inferred current-deck probabilities or remaining-count accounting in this change.
- Do not expand `ACTION_SPACE_SIZE`.

## Decisions

### Add a new profile instead of extending `standard_lite_v2`

`standard_lite_deck_v2` will be a separate profile ID. This lets training and evaluation compare decklist-aware observations against the existing profile without invalidating current `standard_lite_v2` models.

Alternative considered: append the decklist section directly to `standard_lite_v2`. That would make the feature immediately available to default training, but it would also force a schema bump and retraining for every v2 user. The separate profile gives a safer adoption path.

### Encode decklist composition as sorted unique-card rows

The new section will encode one row per unique card ID in the observer's original submitted deck. Rows will be sorted by stable registry index. Each row will include a present flag, card ID, normalized original count, main-deck flag, Digi-Egg flag, and reserved scalar slots.

Alternative considered: encode one row per physical copy. Copy rows would be simpler, but they waste space and make card counts less direct for the policy. Unique-card rows match the user's stated need: card IDs and count within the pilot's deck.

### Store original deck composition in game/player state

Implementation should make original submitted deck composition available from `Game` or each `Player`, not only from `HeadlessRunner`. The observation writer already operates on `Game`, and runner-only metadata would make the profile harder to use from tests, debug runners, and future non-RL callers.

Alternative considered: pass deck IDs into `build_observation_tensor`. That keeps core game state smaller, but it broadens the observation API and creates a parallel state source that can drift from the game setup.

### Keep the profile own-only and orderless

The section will encode only the observer's own original decklist. It MUST NOT expose opponent list composition, current deck order, or face-down security identity. Sorting by registry index avoids carrying submitted order or shuffled order into the tensor.

Alternative considered: include both players' original decklists for supervised matchup experiments. That is useful for some offline analysis but too strong for the default fair-information pilot surface.

### Treat decklist card IDs as card-id positions

The `card_id` field in each decklist row will be included in `card_id_positions` so `CardEmbeddingExtractor` embeds it consistently with hand, permanent, known-zone, and pending-choice card IDs. Count and flags remain scalar positions.

Alternative considered: put decklist card IDs in scalar positions and let the policy learn raw integer magnitudes. That would break the established identity-embedding pattern and make card IDs less meaningful.

## Risks / Trade-offs

- [Risk] The larger tensor increases model input size and training cost. → Mitigation: keep the profile opt-in and compact, with 55 decklist rows sized for 50 main cards plus up to 5 Digi-Eggs.
- [Risk] Original deck metadata can drift from live zones after setup mutations. → Mitigation: capture immutable original counts during game construction before shuffling, drawing, security setup, or mulligan changes.
- [Risk] Row ordering could accidentally encode submitted list order. → Mitigation: require registry-index sorting for unique rows.
- [Risk] Model compatibility gates may accept mismatched artifacts if metadata is incomplete. → Mitigation: update layout metadata, hash, feature schema version, export metadata tests, and compatibility checks for the new profile.
- [Risk] Future users may want remaining-count inference. → Mitigation: reserve row slots and explicitly leave remaining-count semantics out of this profile's first version.

## Migration Plan

1. Add profile metadata and tensor writer support behind the new `standard_lite_deck_v2` profile ID.
2. Keep `standard_lite_v2` as-is so existing models and tests remain valid.
3. Add documentation and tests for the new profile's size, sections, card/scalar positions, PyO3 exports, and Python feature extraction.
4. Train or export new models only when explicitly selecting `standard_lite_deck_v2`.
5. Rollback is removing the new profile selection from training/export callers; existing `standard_lite_v2` remains available throughout.

## Open Questions

- Should `standard_lite_deck_v2` become the default serious-training profile after A/B evaluation, or remain opt-in indefinitely?
- Should a later profile add own-visible remaining-count accounting once the original-composition baseline has been evaluated?
