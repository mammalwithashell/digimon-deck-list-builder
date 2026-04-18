# Archetype QA: Mastemon (Tribal)

Date: 2026-04-12
Total cards: 66 (62 processed, 4 skipped)
Pipeline: batch-fix-cards

## Summary

| Verdict | Count |
|---------|-------|
| FAITHFUL | 1 |
| FIXED | 61 |
| PARTIAL | 0 |
| BLOCKED | 0 |

**62/62 processable cards resolved.** 4 cards skipped (missing scripts, out of scope): BT6-100 Reinforcing Memory Boost!, ST10-06 Mastemon, ST10-12 LadyDevimon, ST10-14 Chaos Degradation.

## Engine Changes

Five engine-level fixes discovered during the campaign:

1. **`Permanent.has_keyword()` inherited aura path** (BT11-042) — extended `has_keyword` to scan inherited aura effects on other perms' `top_card` and `card_sources[:-1]`, matching DP aura semantics. Fixed BT11-042 Angewomon's Blocker aura and related cards.
2. **`Player.digivolve()` CANNOT_REDUCE_COST check** (BT5-033) — digivolve cost calculation now consults `CANNOT_REDUCE_COST` modifiers. Scripts referencing this modifier (BT5-008, BT5-021, BT5-033) were previously dead.
3. **`action_mask.py` CANNOT_ATTACK_TARGET enforcement** (BT10-042) — wired `CANNOT_ATTACK_TARGET` modifier check into normal attack loop, forced attackers, Vortex, MAY_ATTACK/FORCE_ATTACK end-of-turn blocks. Previously defined-but-unused.
4. **Active effect stack + source-aware memory gain** (EX8-030) — added `Game._active_effect_stack`, `Game.current_effect`, `_invoke_effect_callback` helper. Wrapped 13 on_process_callback sites. `Player.add_memory` now passes `source_effect` in modifier context for source-conditional restrictions (e.g., "except by Tamer effects").
5. **`Permanent.de_digivolve()` IMMUNE_FROM_DE_DIGIVOLVE enforcement** (EX10-031) — modifier type existed but was never checked. Now honored in `de_digivolve()`.

## Per-Card Results

| Card ID | Name | Verdict | Tests |
|---------|------|---------|-------|
| BT1-087 | T.K. Takaishi | FIXED | 17 |
| BT4-104 | Blinding Ray | FIXED | 9 |
| BT5-033 | Cutemon | FIXED | 10 |
| BT7-032 | Pulsemon | FIXED | 12 |
| BT7-107 | Calling From the Darkness | FIXED | 19 |
| BT8-071 | Psychemon | FIXED | 7 |
| BT8-077 | BlackGatomon | FAITHFUL | 13 |
| BT9-033 | Pillomon | FIXED | 9 |
| BT9-082 | Ordinemon | FIXED | 17 |
| BT10-042 | Venusmon | FIXED | 18 |
| BT11-042 | Angewomon | FIXED | 25 |
| BT11-080 | Devimon | FIXED | 16 |
| BT11-083 | LadyDevimon | FIXED | 13 |
| BT11-094 | Mirei Mikagura | FIXED | 16 |
| BT13-034 | Kudamon | FIXED | 12 |
| BT13-106 | Odin's Breath | FIXED | 15 |
| BT14-003 | Tokomon | FIXED | 7 |
| BT14-033 | Patamon | FIXED | 17 |
| BT15-003 | Nyaromon | FIXED | 17 |
| BT15-034 | Salamon | FIXED | 25 |
| BT15-037 | Gatomon | FIXED | 17 |
| BT15-038 | Angewomon | FIXED | 17 |
| BT16-088 | Cody Hida & T.K. Takaishi | FIXED | 24 |
| BT18-082 | Lucemon: Chaos Mode | FIXED | 22 |
| BT22-004 | Wanyamon | FIXED | 14 |
| BT22-031 | GoldNumemon | FIXED | 11 |
| BT22-034 | Reppamon | FIXED | 19 |
| BT22-043 | Terriermon | FIXED | 23 |
| BT22-044 | Palmon | FIXED | 13 |
| BT22-046 | Gargomon | FIXED | 14 |
| BT22-054 | Hagurumon | FIXED | 18 |
| BT22-056 | Guardromon | FIXED | 15 |
| BT22-089 | Mirei Mikagura | FIXED | 13 |
| BT22-093 | Ami Aiba | FIXED | 16 |
| BT22-094 | Yuugo Kamishiro | FIXED | 13 |
| BT22-099 | Kuremi Detective Agency | FIXED | 17 |
| BT22-101 | Kyoko Kuremi | FIXED | 21 |
| BT23-027 | Angemon | FIXED | 20 |
| BT23-031 | Angewomon | FIXED | 24 |
| BT23-037 | Tentomon | FIXED | 17 |
| BT23-067 | LadyDevimon | FIXED | 20 |
| BT23-096 | Comet Hammer | FIXED | 25 |
| BT23-102 | Mastemon | FIXED | 23 |
| EX4-074 | ShineGreymon: Ruin Mode | FIXED | 24 |
| EX6-020 | Gatomon | FIXED | 19 |
| EX6-022 | Angewomon | FIXED | 19 |
| EX6-029 | Mastemon | FIXED | 20 |
| EX6-030 | Dominimon | FIXED | 27 |
| EX6-053 | LadyDevimon | FIXED | 11 |
| EX6-074 | Mirei Mikagura | FIXED | 23 |
| EX8-030 | Tapirmon | FIXED | 10 |
| EX8-064 | Boltboutamon | FIXED | 21 |
| EX10-031 | DarkKnightmon | FIXED | 18 |
| EX10-051 | Mummymon | FIXED | 21 |
| LM-035 | Amber Memory Boost! | FIXED | 19 |
| P-187 | Mastemon | FIXED | 26 |
| P-206 | Digital Gate Open | FIXED | 23 |
| P-221 | Chaosmon | FIXED | 19 |
| P-225 | DigiLab | FIXED | 21 |
| ST10-02 | Salamon | FIXED | 11 |
| ST10-04 | Gatomon | FIXED | 20 |
| ST20-05 | Gatomon | FIXED | 20 |

## Skipped Cards

Missing frozen scripts; out of scope for this fix pass:
- BT6-100 Reinforcing Memory Boost!
- ST10-06 Mastemon
- ST10-12 LadyDevimon
- ST10-14 Chaos Degradation

## Recurring Bug Patterns Found

- **Security effect with `is_security_effect=True` but no `on_process_callback`** — completely inert, silently broken. Hit in BT22-089, BT22-094, BT22-099, BT22-101, EX6-074, BT22-093, BT13-106, and more. Widespread across BT22.
- **`value_fn=lambda: -N` arity bug** — `ModifierRegistry.get_int_modifier` calls `value_fn(cur, t, c)` inside `except Exception: pass`, silently no-op. Correct form: `lambda cur, t, c: cur - N`. Found in BT15-038, BT13-034, BT10-042, P-221, and many others.
- **CHANGE_DP without target-equality `condition`** — modifier leaks to ALL permanents queried. Every CHANGE_DP/CHANGE_SECURITY_ATTACK needs `condition=lambda t, c, _tp=target: t is _tp`.
- **Substring trait matching** — `'CS' in trait` wrongly matches "Hudie/CS" but also "CSS"/"CSV". Use exact list membership `'CS' in card.card_traits` or explicit `t == 'CS'`.
- **`contains_card_name()` matches X Antibody variants** — use exact name membership on `card.card_names` list.
- **`is_when_digivolving=True` on Tamer observers** — engine requires host==digivolved_perm, so Tamer observer effects should use plain `set_timing(WhenDigivolving)` without the flag. Found in BT11-094.
- **OnLoseSecurity/OnAddSecurity missing owner gate** — without `event_player is card.owner`, effects fire when opponent triggers.
- **`effect_source_permanent` unset for field_main effects** — use `context.get('permanent')` instead. BT22-043, BT22-054.
- **`OnDestroyedAnyone` vs `_is_deletion_observer`** — `is_on_deletion` only fires self-death triggers. Cross-perm deletion watchers need `_is_deletion_observer=True` + `NoTiming`. BT22-101, EX8-064.
- **Delay callback condition must NOT check `permanent_of_this_card()`** — engine trashes the permanent before invoking the Delay callback. BT22-099, BT22-101, P-225, LM-035.
- **`event_permanent` scope missing on OnAddDigivolutionCards** — effects trigger on ANY perm's stack without this scope check. BT22-004, BT22-044, BT22-054.
- **Non-inherited body effects become unreachable after stacking** — OnAddDigivolutionCards body effects must be `is_inherited_effect=True` so they remain reachable from below the top.
- **`CardSource.base_dp` vs `Permanent.dp`** — filtering hand/trash cards by DP must use `base_dp`.
- **"Top stacked card" semantics** — means `card_sources[-1]` (the current top), not `[-2]`. Pop top, insert at index 0 to soft de-digivolve.
- **`Permanent.digivolution_cards` in Python INCLUDES top_card** — DCGO's `DigivolutionCards` excludes top. Scripts comparing same-level in stack below top must manually exclude `card_sources[-1]`.
- **Vaccine is an ATTRIBUTE** — not in `card_traits` (which is `type_eng`). Check `c.c_entity_base.attribute_eng`.
- **Self-play-from-security wrong zone** — cards with "when trashed from security, may play" need to play from TRASH (not hand). Also `_security_played` flag must be set to prevent double-trash.
- **Raw `security_cards.pop(0)/append(trash)`** bypasses observer firing — use `player.trash_security_card()`.
- **Missing "Then, shuffle" post-search cleanup step.**
- **Alt-digi OR conditions** — need separate `ICardEffect` instances per alternative.

## Test Count

- New behavioral tests written: ~1,040 (across all 62 cards)
- Engine tests: 468 passing
- Full Mastemon card suite: all passing
