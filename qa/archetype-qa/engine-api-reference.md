# Engine API Reference for Card Effect Scripts

This document is the complete reference for writing Digimon TCG card effect scripts.
Every agent implementing or QA-reviewing card effects receives this document.

---

## Table of Contents

1. [Script Structure](#1-script-structure)
2. [ICardEffect API](#2-icardeffect-api)
3. [Game Action Methods](#3-game-action-methods)
4. [Modifier System](#4-modifier-system)
5. [EffectTiming Enum](#5-effecttiming-enum)
6. [Player API](#6-player-api)
7. [Permanent API](#7-permanent-api)
8. [CardSource API](#8-cardsource-api)
9. [Context Dictionary](#9-context-dictionary)
10. [Common Patterns](#10-common-patterns)
11. [Anti-Patterns](#11-anti-patterns)
12. [Complete Examples](#12-complete-examples)

---

## 1. Script Structure

Every card effect script is a Python module in `digimon_gym/engine/data/scripts/{set}/{set}_{number}.py`.

### Boilerplate

```python
from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class SET_NNN(CardScript):
    """SET-NNN Card Name | Type"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ... define effects here ...

        return effects
```

### Class Naming Convention

- Class name: `{SET}_{NNN}` where SET is uppercase set code, NNN is zero-padded number
- Examples: `BT1_090`, `BT23_057`, `EX11_022`, `P_206`
- The class MUST inherit from `CardScript`
- The ONLY method to implement is `get_card_effects(self, card) -> List[ICardEffect]`

### Effect Definition Pattern

Each effect the card has is an `ICardEffect()` instance added to the `effects` list:

```python
effect = ICardEffect()
effect.set_timing(EffectTiming.OnEnterFieldAnyone)  # when it fires
effect.set_effect_name("CARD-ID effect summary")     # short name
effect.set_effect_description("Full card text")       # exact card text
effect.is_on_play = True                              # flag for engine matching

def condition(context: Dict[str, Any]) -> bool:
    # Return True if this effect can activate right now
    return True

effect.set_can_use_condition(condition)

def process(ctx: Dict[str, Any]):
    # Execute the effect's actions
    player = ctx.get('player')
    game = ctx.get('game')
    # ... do things ...

effect.set_on_process_callback(process)
effects.append(effect)
```

---

## 2. ICardEffect API

### Construction

```python
effect = ICardEffect()
```

### Required Setters

| Method | Purpose |
|--------|---------|
| `set_effect_name(name: str)` | Short identifier (e.g., "BT23-057 play cost -5") |
| `set_effect_description(desc: str)` | Full card text for this effect |
| `set_can_use_condition(fn: (Dict) -> bool)` | When can this effect activate? |

### Timing Setters

| Method / Property | Purpose |
|-------------------|---------|
| `set_timing(timing: EffectTiming)` | Set the engine timing hook |
| `is_on_play = True` | Effect fires on play (with `OnEnterFieldAnyone` timing) |
| `is_when_digivolving = True` | Effect fires on digivolve (with `OnEnterFieldAnyone` timing) |
| `is_on_deletion = True` | Effect fires on deletion (with `OnDestroyedAnyone` timing) |
| `is_on_attack = True` | Effect fires on attack (with `OnDeclaration` timing) |

### Callback Setter

| Method | Purpose |
|--------|---------|
| `set_on_process_callback(fn: (Dict) -> None)` | The effect's actual logic |

### OPT (Once Per Turn)

| Method | Purpose |
|--------|---------|
| `set_max_count_per_turn(n: int)` | Limit activations per turn (use 1 for OPT) |
| `set_hash_string(s: str)` | Unique ID for OPT tracking across copies |

### Flags

| Property | Default | Purpose |
|----------|---------|---------|
| `is_optional` | `False` | Player can choose not to activate |
| `is_inherited_effect` | `False` | Effect is gained by cards digivolving on top |
| `is_security_effect` | `False` | Activates from security stack |
| `is_counter_effect` | `False` | Activates during counter timing |
| `is_declarative` | `False` | Passive (no process callback) |
| `is_linked_effect` | `False` | From sideways-linked option card |

### Keyword Flags (Factory Effects)

Set these directly on an effect to grant a keyword to the card:

```python
effect._is_blocker = True
effect._is_rush = True
effect._is_jamming = True
effect._is_piercing = True
effect._is_reboot = True
effect._is_alliance = True
effect._is_scapegoat = True
effect._is_raid = True
effect._is_delay = True
effect._is_cannot_attack = True
effect._is_cannot_block = True
effect._is_cannot_suspend_player = True
```

### Alt-Digi Attributes

For alternate digivolution requirements:

```python
effect._alt_digi_cost = 3       # memory cost
effect._alt_digi_level = 5      # required level of base
effect._alt_digi_trait = "TS"   # required trait (optional)
effect._alt_digi_name = "Omnimon"  # required name (optional)
```

### Cost Reduction

```python
effect.set_timing(EffectTiming.BeforePayCost)
effect.cost_reduction = 5  # reduce play cost by 5
```

### DP Modifier

```python
effect.dp_modifier = 2000  # +2000 DP while on field
```

### Security Attack Modifier

```python
effect._security_attack_modifier = 1  # +1 Security Attack
```

### Other Setters

| Method | Purpose |
|--------|---------|
| `set_effect_source_card(card)` | Set source card (usually not needed — engine sets this) |
| `set_root_card_effect(effect)` | Link to root effect (for sub-effects) |
| `set_can_activate_condition(fn)` | Additional activation gate (rarely needed) |

---

## 3. Game Action Methods

These are the methods card scripts call via `ctx.get('game')` to perform actions.

### Selection Methods

#### `effect_select_opponent_permanent(player, callback, filter_fn=None, is_optional=False, prompt="...")`

Select one of the opponent's permanents.

- `player`: Player making the selection
- `callback(permanent: Permanent)`: Called with selected permanent
- `filter_fn(permanent: Permanent) -> bool`: Optional filter
- `is_optional`: Can player decline?

```python
def process(ctx):
    player = ctx.get('player')
    game = ctx.get('game')

    def on_select(target):
        player.enemy.delete_permanent(target)

    game.effect_select_opponent_permanent(
        player, on_select,
        filter_fn=lambda p: p.is_digimon and p.top_card.get_cost_itself <= 4,
        is_optional=False
    )
```

#### `effect_select_own_permanent(player, callback, filter_fn=None, is_optional=False, prompt="...")`

Same as above but for the player's own permanents.

#### `effect_select_hand_card(player, filter_fn, callback, is_optional=False, prompt="...")`

Select a card from hand.

- `filter_fn(card: CardSource) -> bool`: Required filter

```python
game.effect_select_hand_card(
    player,
    filter_fn=lambda c: c.is_digimon,
    callback=lambda c: player.trash_cards.append(c),
    is_optional=True
)
```

#### `effect_select_own_security(player, filter_fn, callback, is_optional=True, prompt="...")`

Select a card from own security stack.

#### `effect_select_opponent_security(player, filter_fn, callback, is_optional=True, prompt="...")`

Select a card from opponent's security stack.

#### `effect_choose_branch(player, num_choices, callback, prompt="...", branch_labels=None)`

Choose between N effect branches.

- `callback(index: int)`: Called with 0-indexed branch choice

```python
game.effect_choose_branch(
    player, 2,
    callback=lambda choice: do_branch_a() if choice == 0 else do_branch_b(),
    branch_labels=["Draw 2", "Gain 3 memory"]
)
```

### Reveal Methods

#### `effect_reveal_and_select(player, count, filter_fn, on_selected, is_optional=False, prompt="")`

Reveal top N cards from deck, pick one matching filter, rest go to bottom.

- `on_selected(selected: CardSource, remaining: List[CardSource])`: Callback

#### `effect_reveal_and_select_multi(player, count, passes, remaining_placement='deck_bottom', is_optional=False)`

Reveal top N, then run multiple sequential selection passes.

- `passes`: List of `(filter_fn, placement)` tuples
- `placement`: `'hand'`, `'trash'`, or `'deck_bottom'`

```python
game.effect_reveal_and_select_multi(
    player, 5,
    passes=[
        (lambda c: c.is_digimon, 'hand'),   # pick a Digimon to hand
        (lambda c: c.is_option, 'trash'),    # pick an Option to trash
    ],
    remaining_placement='deck_bottom'
)
```

### Play Methods

#### `effect_play_from_zone(player, zone, filter_fn, free=True, manual_reduction=0, is_optional=True, prompt="")`

Play a card from a zone onto the field.

- `zone`: `'hand'`, `'trash'`, `'revealed'`, or `'hand_or_trash'`
- `free`: If True, ignore play cost
- `manual_reduction`: Additional cost reduction

```python
game.effect_play_from_zone(
    player, 'hand_or_trash',
    filter_fn=lambda c: c.is_digimon and any('Puppet' in t for t in getattr(c, 'card_traits', [])),
    free=True, is_optional=True
)
```

#### `effect_digivolve_from_hand(player, permanent, filter_fn, cost_override=None, cost_reduction=0, ignore_requirements=False, is_optional=True, prompt="")`

Digivolve a permanent using a hand card via effect.

- `permanent`: The permanent to digivolve
- `cost_override`: Use this cost instead of calculated
- `cost_reduction`: Reduce from base cost
- `ignore_requirements`: Skip level/color validation

#### `effect_dna_digivolve_from_hand(player, filter_fn, is_optional=True, prompt="")`

Trigger DNA digivolve from hand (2-material combo). Agent picks card then two field targets.

### Token Methods

#### `effect_play_token(player, token_type, on_opponent_field=False, count=1)`

Create and play token permanent(s).

- `token_type`: Token registry key (e.g., `'hinukamuy'`, `'petrification'`, `'diaboromon'`)
- `on_opponent_field`: Place on opponent's field if True

```python
game.effect_play_token(player, "hinukamuy")
```

### Link Methods

#### `effect_link_to_permanent(player, card_to_link, filter_fn=None, is_optional=True, prompt="")`

Link an option card sideways to a Digimon.

### Modifier Registration

#### `register_modifier(modifier_type, target_permanent, condition=None, value_fn=None, source_effect=None, expiry='permanent')`

Register a continuous modifier on a permanent.

- `modifier_type`: `ModifierType` enum value
- `target_permanent`: Permanent to modify
- `condition`: Optional `(permanent, context) -> bool`
- `value_fn`: Optional value function (for DP mods etc.)
- `expiry`: `'permanent'`, `'end_of_turn'`, `'end_of_attack'`, `'end_of_opponent_turn'`

```python
from digimon_gym.engine.interfaces.modifiers import ModifierType

game.register_modifier(
    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
    value_fn=lambda: True, expiry='end_of_turn'
)
```

### Other Game Methods

| Method | Purpose |
|--------|---------|
| `force_end_attack()` | Force current attack to end early |
| `declare_winner(player)` | End game with a winner |

---

## 4. Modifier System

Import: `from digimon_gym.engine.interfaces.modifiers import ModifierType`

### Deletion Prevention

| ModifierType | Effect |
|-------------|--------|
| `CANNOT_BE_DESTROYED` | Cannot be destroyed (blanket) |
| `CANNOT_BE_DESTROYED_BY_BATTLE` | Cannot be destroyed by battle |
| `CANNOT_BE_DESTROYED_BY_EFFECT` | Cannot be destroyed by effects |
| `CANNOT_BE_REMOVED` | Cannot be removed from field |

### Targeting Protection

| ModifierType | Effect |
|-------------|--------|
| `CANNOT_BE_SELECTED_BY_EFFECT` | Cannot be targeted by effects |
| `CANNOT_BE_AFFECTED` | Cannot be affected by any effects |
| `DISABLE_EFFECT` | Permanent's effects are disabled |

### DP Modification

| ModifierType | Effect |
|-------------|--------|
| `CHANGE_DP` | Modify current DP |
| `CHANGE_BASE_DP` | Modify base DP |
| `CHANGE_CARD_DP` | Modify DP from card properties |
| `IMMUNE_FROM_DP_MINUS` | Cannot lose DP |
| `DONT_HAVE_DP` | Permanent has 0 DP |
| `CHANGE_DP_DELETE_MAX` | Change max DP for deletion |

### Cost Modification

| ModifierType | Effect |
|-------------|--------|
| `CHANGE_PLAY_COST` | Modify play cost |
| `CHANGE_DIGIVOLUTION_COST` | Modify digivolution cost |
| `CANNOT_REDUCE_COST` | Block cost reduction effects |

### Security Attack

| ModifierType | Effect |
|-------------|--------|
| `CHANGE_SECURITY_ATTACK` | Modify security attack value |
| `INVERT_SECURITY_ATTACK` | Negate security attack |

### Suspend / Unsuspend

| ModifierType | Effect |
|-------------|--------|
| `CANNOT_SUSPEND` | Cannot be suspended |
| `CANNOT_UNSUSPEND` | Cannot be unsuspended |

### Movement Prevention

| ModifierType | Effect |
|-------------|--------|
| `CANNOT_RETURN_TO_HAND` | Cannot bounce to hand |
| `CANNOT_RETURN_TO_DECK` | Cannot return to deck |
| `CANNOT_MOVE` | Cannot move slots |

### Play / Field Restrictions

| ModifierType | Effect |
|-------------|--------|
| `CANNOT_PLAY_CARD` | Prevent cards from being played |
| `CANNOT_PUT_ON_FIELD` | Prevent permanents entering field |
| `CANNOT_DIGIVOLVE` | Cannot digivolve |

### Attack Restrictions

| ModifierType | Effect |
|-------------|--------|
| `FORCE_ATTACK` | Must attack at start of main |
| `CANNOT_ATTACK` | Cannot attack |
| `CANNOT_ATTACK_TARGET` | Cannot attack specific target |
| `CAN_ATTACK_TARGET` | Can attack normally untargetable permanent |
| `CAN_ATTACK_UNSUSPENDED` | Can attack unsuspended Digimon |
| `CANNOT_SWITCH_ATTACK_TARGET` | Attack target cannot be changed |
| `CANNOT_BLOCK` | Cannot block |

### Attribute Overrides

| ModifierType | Effect |
|-------------|--------|
| `CHANGE_CARD_NAMES` | Modify card name |
| `CHANGE_BASE_CARD_NAMES` | Modify base card name |
| `CHANGE_CARD_COLORS` | Modify colors |
| `CHANGE_BASE_CARD_COLORS` | Modify base colors |
| `CHANGE_TRAITS` | Modify traits |
| `CHANGE_PERMANENT_LEVEL` | Modify level on field |
| `CHANGE_CARD_LEVEL` | Modify base level |

### Keyword Grants (via Modifier)

| ModifierType | Effect |
|-------------|--------|
| `GRANT_BLOCKER` | Grant Blocker keyword |
| `GRANT_RUSH` | Grant Rush keyword |
| `GRANT_REBOOT` | Grant Reboot keyword |
| `GRANT_ALLIANCE` | Grant Alliance keyword |
| `GRANT_ICECLAD` | Grant Iceclad keyword |
| `GRANT_SCAPEGOAT` | Grant Scapegoat keyword |
| `TREAT_AS_DIGIMON` | Treat as Digimon type |
| `ADD_SKILL` | Add skill to permanent |

### Other Modifiers

| ModifierType | Effect |
|-------------|--------|
| `CANNOT_ADD_MEMORY` | Cannot gain memory |
| `CANNOT_ADD_SECURITY` | Cannot add to security |
| `CHANGE_END_TURN_MIN_MEMORY` | Modify min memory at end of turn |
| `IMMUNE_FROM_DE_DIGIVOLVE` | Cannot be de-digivolved |
| `IMMUNE_FROM_STACK_TRASHING` | Digi-stack cannot be trashed |
| `CANNOT_TRASH_DIGIVOLUTION_CARDS` | Cannot trash from digi-stack |
| `DONT_BATTLE_SECURITY_DIGIMON` | Skip security Digimon battle |
| `CHANGE_LINK_MAX` | Modify max link count |
| `VORTEX_CAN_ATTACK_PLAYERS` | Vortex can attack players |

### Expiry Types

| Value | Clears When |
|-------|-------------|
| `'permanent'` | When source permanent leaves field (default) |
| `'end_of_turn'` | At end of current turn |
| `'end_of_attack'` | At end of current attack |
| `'end_of_opponent_turn'` | At end of opponent's next turn |

---

## 5. EffectTiming Enum

Import: `from digimon_gym.engine.data.enums import EffectTiming`

### Card Event Timings

| Value | Fires When |
|-------|------------|
| `OnEnterFieldAnyone` (3) | Any permanent enters the field |
| `OnDestroyedAnyone` (6) | Any permanent is destroyed |
| `OnTappedAnyone` (29) | Any permanent is suspended |
| `OnUnTappedAnyone` (30) | Any permanent is unsuspended |
| `OnDeclaration` (2) | An attack is declared |
| `OnBlockAnyone` (34) | Any permanent blocks |
| `OnGetDamage` (4) | A player takes damage |
| `OnDraw` (17) | A card is drawn |
| `OnAddHand` (18) | A card is added to hand |
| `OnDiscardHand` (22) | A card is discarded from hand |
| `OnMove` (26) | A permanent moves |
| `OnKnockOut` (25) | A permanent is knocked out |

### Phase Timings

| Value | Fires When |
|-------|------------|
| `OnStartTurn` (15) | Start of turn |
| `OnStartMainPhase` (39) | Start of main phase |
| `OnEndMainPhase` (16) | End of main phase |
| `OnStartBattle` (40) | Start of battle |
| `OnEndBattle` (41) | End of battle |
| `OnEndAttack` (43) | End of attack |
| `OnEndAttackPhase` (13) | End of attack phase |
| `OnEndTurn` (14) | End of turn |

### Security Timings

| Value | Fires When |
|-------|------------|
| `OnSecurityCheck` (35) | Security is checked |
| `SecuritySkill` (38) | Security card skill activates |
| `OnLoseSecurity` (19) | Security card is lost |
| `OnAddSecurity` (20) | Security card is added |
| `OnDiscardSecurity` (23) | Security card is discarded |

### Cost Timings

| Value | Fires When |
|-------|------------|
| `BeforePayCost` (44) | Before play cost is paid |
| `AfterPayCost` (45) | After play cost is paid |

### Digivolution Timings

| Value | Fires When |
|-------|------------|
| `WhenDigivolving` (58) | During digivolution |
| `WhenWouldDigivolve` (57) | Before digivolution happens |
| `WhenDigisorption` (7) | Digisorption is used |
| `OnAddDigivolutionCards` (31) | Cards added to digi-stack |
| `OnDigivolutionCardDiscarded` (46) | Digi-card is discarded |
| `WhenWouldDigivolutionCardDiscarded` (52) | Before digi-card would be discarded |
| `OnDigivolutionCardReturnToDeckBottom` (47) | Digi-card returns to deck bottom |
| `WhenTopCardTrashed` (54) | Top card is trashed |

### Removal Timings

| Value | Fires When |
|-------|------------|
| `WhenRemoveField` (8) | When a permanent would be removed from field |
| `OnRemovedField` (56) | After a permanent is removed from field |
| `WhenPermanentWouldBeDeleted` (9) | Before permanent would be deleted |
| `WhenReturntoHandAnyone` (11) | Any card returns to hand |
| `WhenReturntoLibraryAnyone` (10) | Any card returns to library |
| `OnPermamemtReturnedToHand` (49) | Permanent returned to hand |
| `OnReturnCardsToHandFromTrash` (50) | Card returns to hand from trash |
| `OnReturnCardsToLibraryFromTrash` (48) | Card returns to library from trash |

### Special Timings

| Value | Fires When |
|-------|------------|
| `OptionSkill` (5) | Option card main effect |
| `OnUseOption` (1) | When an option card is used |
| `OnUseAttack` (28) | When an attack is used |
| `OnAllyAttack` (32) | When an ally attacks |
| `OnCounterTiming` (33) | During counter timing |
| `OnAttackTargetChanged` (36) | Attack target changes |
| `OnEndBlockDesignation` (37) | End of block designation |
| `OnDiscardLibrary` (24) | Card discarded from library |
| `OnUseDigiburst` (21) | Digiburst is used |
| `OnEndCoinToss` (27) | End of coin toss |
| `AfterEffectsActivate` (51) | After all effects resolve |
| `WhenLinked` (53) | Cards are linked |
| `RulesTiming` (55) | Rules engine timing hook |
| `OnDetermineDoSecurityCheck` (42) | Determining if security check occurs |
| `NoTiming` (0) | Placeholder (factory effects, keywords) |

---

## 6. Player API

Access via `ctx.get('player')`.

### Memory & Draw

| Method | Effect |
|--------|--------|
| `add_memory(amount)` | Gain memory |
| `lose_memory(amount)` | Lose memory |
| `draw()` | Draw 1 card |
| `draw_cards(count)` | Draw N cards |
| `recovery(count)` | Move N deck cards to security |
| `mill(count) -> List[CardSource]` | Trash N from deck top |

### Hand Operations

| Method | Effect |
|--------|--------|
| `trash_from_hand(cards: List[CardSource])` | Move cards from hand to trash |

### Permanent Operations

| Method | Effect |
|--------|--------|
| `delete_permanent(perm, is_battle=False, is_opponent_effect=False)` | Delete a permanent |
| `bounce_permanent_to_hand(perm)` | Return top card to hand, rest to trash |
| `return_permanent_to_deck_bottom(perm)` | Return top card to deck bottom |
| `put_permanent_to_security(perm, face_up=False)` | Move permanent to security |
| `hatch()` | Move top Digitama to breeding area |
| `move_from_breeding()` | Move from breeding to battle area |

### Security Operations

| Method | Effect |
|--------|--------|
| `add_to_security_from_hand(card, to_top=True)` | Hand card to security |
| `trash_security_card(card)` | Remove and trash from security |

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `battle_area` | `List[Permanent]` | All field permanents |
| `breeding_area` | `Optional[Permanent]` | Breeding slot |
| `hand_cards` | `List[CardSource]` | Hand |
| `library_cards` | `List[CardSource]` | Deck |
| `security_cards` | `List[CardSource]` | Security stack |
| `trash_cards` | `List[CardSource]` | Trash pile |
| `digitama_library_cards` | `List[CardSource]` | Egg deck |
| `enemy` | `Player` | Opponent |
| `is_my_turn` | `bool` | Is this player's turn? |
| `memory` | `int` | Current memory |

---

## 7. Permanent API

Access via `ctx.get('permanent')` or from selection callbacks.

### Identity

| Property | Type | Description |
|----------|------|-------------|
| `top_card` | `CardSource` | Topmost card in stack |
| `card_sources` | `List[CardSource]` | Full digi-stack (bottom to top) |
| `is_digimon` | `bool` | Top card is Digimon |
| `is_tamer` | `bool` | Top card is Tamer |
| `is_option` | `bool` | Top card is Option |
| `is_token` | `bool` | Is a token |
| `level` | `Optional[int]` | Level |
| `dp` | `Optional[int]` | Current DP (after modifiers) |
| `is_suspended` | `bool` | Is suspended/tapped |
| `has_no_digivolution_cards` | `bool` | Only 1 card in stack |
| `linked_cards` | `List[CardSource]` | Sideways-linked option cards |

### State Modification

| Method | Effect |
|--------|--------|
| `suspend()` | Suspend, fires OnTappedAnyone |
| `unsuspend()` | Unsuspend, fires OnUnTappedAnyone |
| `change_dp(amount)` | Temporary DP change (until end of turn) |
| `grant_keyword(keyword_attr)` | Grant a keyword (e.g., `'_is_rush'`) |
| `de_digivolve(count) -> List[CardSource]` | Remove top N cards from stack, return removed |
| `trash_digivolution_cards(count, from_top=True) -> List[CardSource]` | Trash N under-cards |
| `add_card_source(card)` | Add card to top of stack |
| `add_card_source_bottom(card)` | Add card to bottom of stack |
| `link_card(card)` | Link option card sideways |
| `unlink_all() -> List[CardSource]` | Remove all linked cards |

### Checks

| Method | Effect |
|--------|--------|
| `contains_card_name(name) -> bool` | Top card name contains substring (case-insensitive) |
| `has_trait(trait) -> bool` | Top card has this trait |
| `has_keyword(attr) -> bool` | Has keyword (e.g., `'_is_blocker'`) |
| `can_attack() -> bool` | Can this permanent attack? |
| `can_block(attacker) -> bool` | Can this block the attacker? |

---

## 8. CardSource API

Access via `card` parameter in `get_card_effects`, or from hand/trash selection callbacks.

### Identity

| Property | Type | Description |
|----------|------|-------------|
| `card_id` | `str` | Card ID (e.g., "BT23-057") |
| `card_names` | `List[str]` | Card name(s) |
| `card_text` | `str` | Full effect text |
| `is_digimon` | `bool` | Is Digimon kind |
| `is_tamer` | `bool` | Is Tamer kind |
| `is_option` | `bool` | Is Option kind |
| `is_digi_egg` | `bool` | Is DigiEgg kind |
| `is_token` | `bool` | Is token |
| `card_colors` | `List[CardColor]` | Colors |
| `card_traits` | `List[str]` | Traits |
| `level` | `Optional[int]` | Level |
| `base_dp` | `Optional[int]` | Base DP |
| `get_cost_itself` | `int` (property) | Play cost |
| `owner` | `Player` | Owning player |

### Key Methods

| Method | Returns | Purpose |
|--------|---------|---------|
| `permanent_of_this_card()` | `Optional[Permanent]` | Find which Permanent contains this card on field |
| `contains_card_name(name)` | `bool` | Name contains substring |

---

## 9. Context Dictionary

Both condition and process callbacks receive a `context: Dict[str, Any]`.

### Standard Keys (always present)

| Key | Type | Description |
|-----|------|-------------|
| `game` | `Game` | Game instance |
| `player` | `Player` | Effect owner's player |
| `permanent` | `Permanent` | Permanent hosting this effect (if on field) |
| `card` | `CardSource` | The card this effect belongs to |
| `turn_player` | `Player` | Current turn player |
| `opponent_player` | `Player` | Opponent of effect owner |

### Event-Specific Keys (present depending on timing)

| Key | Present When | Type | Description |
|-----|-------------|------|-------------|
| `played_card` | OnEnterFieldAnyone | `CardSource` | Card that was just played |
| `digivolved_permanent` | WhenDigivolving | `Permanent` | Permanent that just digivolved |
| `event_player` | Various | `Player` | Player who triggered the event |
| `event_permanent` | Various | `Permanent` | Permanent that triggered the event |
| `card_source` | BeforePayCost | `CardSource` | Card whose cost is being calculated |
| `deleted_permanent` | OnDestroyedAnyone | `Permanent` | Permanent that was deleted |
| `attacker` | OnDeclaration | `Permanent` | Attacking permanent |

---

## 10. Common Patterns

### Pattern 1: On Play / When Digivolving (same effect, two triggers)

Many cards have the same effect for both triggers. Use a factory function:

```python
def make_effect(name, when_digivolving):
    effect = ICardEffect()
    effect.set_timing(EffectTiming.OnEnterFieldAnyone)
    effect.set_effect_name(name)
    effect.set_effect_description("...")
    if when_digivolving:
        effect.is_when_digivolving = True
    else:
        effect.is_on_play = True

    def condition(context):
        return bool(card and card.permanent_of_this_card() is not None)
    effect.set_can_use_condition(condition)

    def process(ctx):
        # ... shared logic ...
        pass
    effect.set_on_process_callback(process)
    return effect

effects.append(make_effect("CARD-ID effect", when_digivolving=False))
effects.append(make_effect("CARD-ID effect", when_digivolving=True))
```

### Pattern 2: Cost Reduction with Leak Guard

**CRITICAL**: Every BeforePayCost effect MUST check `context.get('card_source') is card` or
`context.get('card_source') is not card` to prevent cost reduction from leaking to other cards.

```python
effect.set_timing(EffectTiming.BeforePayCost)
effect.cost_reduction = 5
effect.set_hash_string("PlayCost-5_BT23_057")

def condition(context):
    if context.get('card_source') is not card:
        return False  # LEAK GUARD — only reduce cost for THIS card
    owner = card.owner if card else None
    if not owner:
        return False
    # ... additional conditions ...
    return True
effect.set_can_use_condition(condition)
```

### Pattern 3: OPT (Once Per Turn) with Hash String

```python
effect.set_max_count_per_turn(1)
effect.set_hash_string("UniqueId_CARD_ID")  # must be globally unique
```

### Pattern 4: Field Presence Check

For effects that only work while the card is on the field:

```python
def condition(context):
    if card and card.permanent_of_this_card() is None:
        return False
    return True
```

### Pattern 5: "This Digimon" Trigger Filtering

For "When THIS Digimon suspends/attacks/etc." — check that the event permanent matches:

```python
def condition(context):
    if card and card.permanent_of_this_card() is None:
        return False
    ctx_perm = context.get('permanent')
    owner_perm = card.permanent_of_this_card()
    if owner_perm and ctx_perm and ctx_perm is not owner_perm:
        return False  # not THIS digimon
    return True
```

### Pattern 6: Inherited Effects

Effects below the line on a card (inherited by Digimon that digivolve on top):

```python
effect.is_inherited_effect = True
# Everything else is the same — condition, process, timing
```

### Pattern 7: Delete Opponent Permanent

```python
def process(ctx):
    player = ctx.get('player')
    game = ctx.get('game')
    if not (player and game):
        return

    def on_delete(target):
        enemy = player.enemy
        if enemy:
            enemy.delete_permanent(target)

    game.effect_select_opponent_permanent(
        player, on_delete,
        filter_fn=lambda p: p.is_digimon,
        is_optional=False
    )
```

### Pattern 8: Trait / Name / Color Filtering

```python
# Check trait on a CardSource
any('Puppet' in t for t in getattr(c, 'card_traits', []))

# Check name on a CardSource
any('Jesmon' in n for n in getattr(c, 'card_names', []))

# Check color on a CardSource
from digimon_gym.engine.data.enums import CardColor
CardColor.Blue in getattr(c, 'card_colors', [])

# Check trait on a Permanent's top card
any('TS' in t for t in getattr(perm.top_card, 'card_traits', []))
```

### Pattern 9: Keyword Factory Effect

Keywords that the card inherently has (not granted via modifier):

```python
effect = ICardEffect()
effect.set_effect_name("CARD-ID Blocker")
effect.set_effect_description("Blocker")
effect._is_blocker = True

def condition(context):
    return True
effect.set_can_use_condition(condition)
effects.append(effect)
```

### Pattern 10: Register Modifier with Expiry

```python
from digimon_gym.engine.interfaces.modifiers import ModifierType

# Grant targeting protection until end of turn
game.register_modifier(
    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
    value_fn=lambda: True, expiry='end_of_turn'
)

# Grant +3000 DP until end of opponent's turn
game.register_modifier(
    ModifierType.CHANGE_DP, target_perm,
    value_fn=lambda: 3000, expiry='end_of_opponent_turn'
)
```

### Pattern 11: Play Cards from Digivolution Stack

Some effects play cards from a permanent's digivolution sources. Access `perm.card_sources`,
filter, remove from stack, then call `player.play_card_from_source()`.

```python
def process(ctx):
    player = ctx.get('player')
    perm = ctx.get('permanent')
    game = ctx.get('game')
    if not (player and perm and game):
        return

    top = perm.top_card
    for cs in list(perm.card_sources):
        if cs is top:
            continue  # never play the top card itself
        if not getattr(cs, 'is_digimon', False):
            continue
        level = getattr(cs, 'level', None)
        if level is None or level > 4:
            continue
        colors = [c.name for c in (getattr(cs, 'card_colors', None) or [])]
        if 'Blue' not in colors:
            continue

        # Remove from digi-stack, then play
        perm.card_sources.remove(cs)
        played = player.play_card_from_source(cs, pay_cost=False)
        if played:
            game.execute_effects(
                EffectTiming.OnEnterFieldAnyone,
                {"played_card": cs, "played_permanent": played,
                 "event_player": player},
            )
        break  # only play 1
```

**Critical rules for this pattern:**
- Always skip `top_card` — it IS the permanent
- Remove from `card_sources` BEFORE calling `play_card_from_source`
- Fire `OnEnterFieldAnyone` effects after playing
- Use `getattr` for safe attribute access on CardSource

### Pattern 12: [Hand][Main] Effect

Cards with `[Hand][Main]` effects — abilities activated from hand during Main phase that aren't
standard play or digivolve actions (e.g., conditional digivolve with side-costs, place-under,
DP buff from hand). Uses action range 30-59 (`30 + hand_idx`).

#### Skeleton

```python
effect0 = ICardEffect()
effect0._is_hand_main = True
effect0.set_effect_name("CARD-ID [Hand][Main] description")
effect0.set_effect_description("[Hand] [Main] ...")

def condition0(context):
    if card.permanent_of_this_card() is not None:
        return False  # must be in hand
    player = card.owner
    if not player or not player.is_my_turn:
        return False
    # Card-specific checks (tamer in play, cards in trash, target on field, etc.)
    ...
    return True

effect0.set_can_use_condition(condition0)

def process0(ctx):
    game, player = ctx['game'], ctx['player']
    hand_card = ctx['card']       # the CardSource being activated
    hand_idx = ctx['hand_idx']    # index in player.hand_cards
    # Full resolution: select targets, place cards, digivolve, pay costs
    ...

effect0.set_on_process_callback(process0)
effects.append(effect0)
```

#### Key rules

- Condition IS checked during masking (unlike `_alt_digi`)
- Always check `card.permanent_of_this_card() is not None → False` (must be in hand)
- Always check `player.is_my_turn`
- The process callback handles ALL resolution logic (target selection, cost payment, side effects)
- Context dict keys: `'game'`, `'player'`, `'card'` (the CardSource), `'hand_idx'` (position in hand), `'permanent'` (always `None`), `'turn_player'`, `'opponent_player'`

#### Resolution patterns

| Pattern | Process callback flow |
|---------|----------------------|
| Conditional digivolve | Select trash card → select field target → `add_card_source_bottom` (place under) → `add_card_source` (digivolve) → `lose_memory` → `draw` → fire `WhenDigivolving` |
| Place self under target | `effect_select_own_permanent` → remove from hand → `add_card_source_bottom` → `lose_memory` |
| Legend-Arms DP buff | `effect_select_own_permanent` → remove from hand → `add_card_source_bottom` → `register_modifier(CHANGE_DP, ...)` |
| Conditional play | Remove from hand → `play_card_from_source(card, pay_cost=...)` → register end-of-turn deletion |
| Tamer hybrid | Select trash cards → select tamer → place under → digivolve |

#### Selecting from trash inside a process callback

When a [Hand][Main] effect needs the agent to choose a card from trash (e.g., which
[Dimetromon] to place), use `game.request_selection` with `GamePhase.SelectTrash`:

```python
_SEL_TRASH_START = 130  # mirrors constants.SEL_TRASH_START

valid_trash = []
for i, c in enumerate(player.trash_cards):
    if my_filter(c):
        valid_trash.append(_SEL_TRASH_START + i)
if not valid_trash:
    return

def on_trash_selected(action_id):
    idx = action_id - _SEL_TRASH_START
    chosen = player.trash_cards[idx]
    # ... use chosen card ...

game.request_selection(
    GamePhase.SelectTrash, player, on_trash_selected,
    valid_trash, is_optional=False,
    prompt="Select a card from your trash.")
```

#### Complete example: BT24-016 Lamiamon (conditional digivolve with trash placement)

Reference implementation in `scripts/bt24/bt24_016.py`. Effect text:
> [Hand] [Main] If you have [Owen Dreadnought], by placing 1 [Dimetromon]
> from your trash as any of your [Elizamon]'s bottom digivolution card,
> it digivolves into this card for a cost of 3, ignoring requirements.

```python
effect0 = ICardEffect()
effect0._is_hand_main = True
effect0.set_effect_name("BT24-016 [Hand][Main] Place Dimetromon, digivolve onto Elizamon")
effect0.set_effect_description("[Hand] [Main] ...")

def condition0(context):
    if card.permanent_of_this_card() is not None:
        return False
    player = card.owner
    if not player or not player.is_my_turn:
        return False
    has_owen = any(p.contains_card_name('Owen Dreadnought') for p in player.battle_area)
    if not has_owen:
        return False
    has_dimetromon_in_trash = any(
        any('Dimetromon' in (n or '') for n in (getattr(c, 'card_names', []) or []))
        for c in player.trash_cards
    )
    if not has_dimetromon_in_trash:
        return False
    has_elizamon = any(p.contains_card_name('Elizamon') for p in player.battle_area)
    return has_elizamon

effect0.set_can_use_condition(condition0)

def process0(ctx):
    game, player, hand_card = ctx.get('game'), ctx.get('player'), ctx.get('card')
    if not (game and player and hand_card):
        return

    def _is_dimetromon(c):
        names = getattr(c, 'card_names', []) or []
        return any('Dimetromon' in (n or '') for n in names)

    # Step 1: Agent chooses which Dimetromon from trash
    def on_dimetromon_selected(trash_idx):
        if trash_idx >= len(player.trash_cards):
            return
        dimetromon = player.trash_cards[trash_idx]

        # Step 2: Agent chooses which Elizamon on field
        def on_elizamon_selected(target_perm):
            if dimetromon in player.trash_cards:
                player.trash_cards.remove(dimetromon)
            target_perm.add_card_source_bottom(dimetromon)

            if hand_card in player.hand_cards:
                player.hand_cards.remove(hand_card)
            target_perm.add_card_source(hand_card)
            target_perm.turn_digivolved = game.turn_count
            player.lose_memory(3)
            player.draw()
            game.execute_effects(EffectTiming.WhenDigivolving,
                                 {"digivolved_permanent": target_perm})

        game.effect_select_own_permanent(
            player, on_elizamon_selected,
            filter_fn=lambda p: p.contains_card_name('Elizamon'),
            is_optional=False)

    _SEL_TRASH_START = 130
    valid_trash = [_SEL_TRASH_START + i for i, c in enumerate(player.trash_cards)
                   if _is_dimetromon(c)]
    if not valid_trash:
        return

    def _on_trash_action(action_id):
        on_dimetromon_selected(action_id - _SEL_TRASH_START)

    game.request_selection(
        GamePhase.SelectTrash, player, _on_trash_action,
        valid_trash, is_optional=False,
        prompt="Select a [Dimetromon] from your trash.")

effect0.set_on_process_callback(process0)
```

### Pattern 13: Partition (WhenRemoveField + Play from Trash)

Partition fires when the permanent would leave the field. By the time WhenRemoveField fires,
the cards are already in `player.trash_cards`. Find them there and play them.

```python
effect = ICardEffect()
effect.set_timing(EffectTiming.WhenRemoveField)
effect.is_optional = True
effect._is_partition = True

def condition(context):
    if card and card.permanent_of_this_card() is None:
        return False
    # Check trash for required cards (they were just trashed)
    owner = card.owner
    if not owner:
        return False
    blue_lv4 = any(
        getattr(c, 'level', None) == 4
        and 'Blue' in [cl.name for cl in (getattr(c, 'card_colors', None) or [])]
        for c in owner.trash_cards
    )
    green_lv4 = any(
        getattr(c, 'level', None) == 4
        and 'Green' in [cl.name for cl in (getattr(c, 'card_colors', None) or [])]
        for c in owner.trash_cards
    )
    return blue_lv4 and green_lv4

effect.set_can_use_condition(condition)

def process(ctx):
    player = ctx.get('player')
    game = ctx.get('game')
    if not (player and game):
        return
    # Play 1 Blue Lv.4 from trash
    for c in list(player.trash_cards):
        if (getattr(c, 'level', None) == 4
                and getattr(c, 'is_digimon', False)
                and 'Blue' in [cl.name for cl in (getattr(c, 'card_colors', None) or [])]):
            player.trash_cards.remove(c)
            played = player.play_card_from_source(c, pay_cost=False)
            if played:
                game.execute_effects(EffectTiming.OnEnterFieldAnyone,
                    {"played_card": c, "played_permanent": played, "event_player": player})
            break
    # Play 1 Green Lv.4 from trash
    for c in list(player.trash_cards):
        if (getattr(c, 'level', None) == 4
                and getattr(c, 'is_digimon', False)
                and 'Green' in [cl.name for cl in (getattr(c, 'card_colors', None) or [])]):
            player.trash_cards.remove(c)
            played = player.play_card_from_source(c, pay_cost=False)
            if played:
                game.execute_effects(EffectTiming.OnEnterFieldAnyone,
                    {"played_card": c, "played_permanent": played, "event_player": player})
            break

effect.set_on_process_callback(process)
```

---

## 11. Anti-Patterns

### NEVER Do These

1. **Never stub an effect.** If you cannot faithfully implement an effect, report BLOCKED. Do not
   write `pass # TODO` or approximate the behavior.

2. **Never omit the leak guard on cost reduction.** Every `BeforePayCost` condition MUST check
   `context.get('card_source') is not card: return False` as its first line.

3. **Never skip the field presence check.** Effects that require the card to be on field MUST check
   `card.permanent_of_this_card() is not None` in their condition.

4. **Never use `player.field_cards`** — it doesn't exist. Use `player.battle_area`.

5. **Never import game/player/permanent at module level.** Use `TYPE_CHECKING` guard:
   ```python
   if TYPE_CHECKING:
       from ....core.card_source import CardSource
   ```

6. **Never hardcode player references.** Always use `ctx.get('player')` and `player.enemy`.

7. **Never forget inherited versions.** If card text has effects below the inheritance line,
   create separate `ICardEffect` instances with `is_inherited_effect = True`.

8. **Never share mutable state between effects.** Each effect's condition/process closures should
   only close over `card` (the CardSource parameter) and the effect variable itself.

9. **Never call `game.execute_effects()` from a card script.** The engine manages timing resolution.
   Card scripts only define individual effects.

10. **Never modify `card_sources` directly.** Use `permanent.add_card_source()`,
    `permanent.de_digivolve()`, etc.

---

## 12. Complete Examples

### Example A: Simple Option Card

BT1-090 Gravity Crush — "[Main] Gain 2 memory."

```python
from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT1_090(CardScript):
    """BT1-090 Gravity Crush"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT1-090 Gain 2 memory")
        effect0.set_effect_description("[Main] Gain 2 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(2)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
```

### Example B: Cost Reduction with Leak Guard + Token Play

BT23-057 Gankoomon — Alternate digivolution, cost reduction requiring trash cards,
on play/when digivolving token spawn + deletion.

```python
from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_057(CardScript):
    """BT23-057 Gankoomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Alt-digi: Lv.5 for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-057 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Cost reduction: -5 if 3+ Huckmon/Sistermon/Jesmon in trash
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT23-057 play cost -5")
        effect1.set_effect_description(
            "When this card would be played, by returning 3 cards with "
            "[Huckmon], [Sistermon] or [Jesmon] in their names from your "
            "trash to the top or bottom of the deck, reduce the play cost by 5."
        )
        effect1.set_hash_string("PlayCost-5_BT23_057")
        effect1.cost_reduction = 5

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False  # LEAK GUARD
            owner = card.owner if card else None
            if not owner:
                return False
            qualifying = [c for c in owner.trash_cards
                          if any('Huckmon' in n or 'Sistermon' in n or 'Jesmon' in n
                                 for n in getattr(c, 'card_names', []))]
            return len(qualifying) >= 3

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            returned = 0
            for c in list(player.trash_cards):
                if returned >= 3:
                    break
                names = getattr(c, 'card_names', [])
                if any('Huckmon' in n or 'Sistermon' in n or 'Jesmon' in n
                       for n in names):
                    player.trash_cards.remove(c)
                    player.library_cards.append(c)
                    returned += 1

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # On Play / When Digivolving: play token + delete opponent
        def make_token_delete_effect(name, when_digivolving):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name(name)
            effect.set_effect_description(
                "You may play 1 [Hinukamuy] Token. Then, delete 1 of your "
                "opponent's Digimon within the effect's play-cost cap."
            )
            effect.is_optional = True
            if when_digivolving:
                effect.is_when_digivolving = True
            else:
                effect.is_on_play = True

            def condition(context: Dict[str, Any]) -> bool:
                return bool(card and card.permanent_of_this_card() is not None)
            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get("player")
                game = ctx.get("game")
                owner_perm = ctx.get("permanent")
                if not (player and game and owner_perm):
                    return

                game.effect_play_token(player, "hinukamuy")
                max_cost = 6 + (3 * sum(
                    1 for p in player.battle_area
                    if p is not owner_perm and p.is_digimon
                ))

                def target_filter(p):
                    if not p.is_digimon or not p.top_card:
                        return False
                    return p.top_card.get_cost_itself <= max_cost

                def on_delete(target_perm):
                    enemy = player.enemy
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=target_filter, is_optional=False
                )

            effect.set_on_process_callback(process)
            return effect

        effects.append(make_token_delete_effect(
            "BT23-057 play Hinukamuy Token", when_digivolving=False))
        effects.append(make_token_delete_effect(
            "BT23-057 play Hinukamuy Token", when_digivolving=True))

        return effects
```

### Example C: Blocker + Inherited Effects + De-Digivolve on Suspend

BT23-077 Sistermon Ciel — Blocker keyword, on-play delete, when-suspends
de-digivolve. All three effects are also inherited.

```python
from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_077(CardScript):
    """BT23-077 Sistermon Ciel | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Blocker keyword
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("BT23-077 Blocker")
        effect_blocker.set_effect_description("Blocker")
        effect_blocker._is_blocker = True
        def cond_blocker(context):
            return True
        effect_blocker.set_can_use_condition(cond_blocker)
        effects.append(effect_blocker)

        # [On Play] Delete 1 of opponent's Digimon with play cost <= 4
        effect_delete = ICardEffect()
        effect_delete.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_delete.set_effect_name("BT23-077 Delete 1 Digimon cost <= 4")
        effect_delete.set_effect_description(
            "[On Play] Delete 1 of your opponent's Digimon with a play cost of 4 or less."
        )
        effect_delete.is_on_play = True

        def cond_delete(context):
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_delete.set_can_use_condition(cond_delete)

        def process_delete(ctx):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_del(target):
                if player.enemy:
                    player.enemy.delete_permanent(target)
            game.effect_select_opponent_permanent(
                player, on_del,
                filter_fn=lambda p: p.is_digimon and p.top_card and p.top_card.get_cost_itself <= 4,
                is_optional=False
            )
        effect_delete.set_on_process_callback(process_delete)
        effects.append(effect_delete)

        # [All Turns] When this Digimon suspends, De-Digivolve 1
        effect_dedigivolve = ICardEffect()
        effect_dedigivolve.set_timing(EffectTiming.OnTappedAnyone)
        effect_dedigivolve.set_effect_name("BT23-077 De-Digivolve 1 on suspend")
        effect_dedigivolve.set_effect_description(
            "[All Turns] When this Digimon suspends, <De-Digivolve 1> "
            "1 of your opponent's Digimon."
        )

        def cond_dedigivolve(context):
            if card and card.permanent_of_this_card() is None:
                return False
            ctx_perm = context.get('permanent')
            owner_perm = card.permanent_of_this_card()
            if owner_perm and ctx_perm and ctx_perm is not owner_perm:
                return False
            return True
        effect_dedigivolve.set_can_use_condition(cond_dedigivolve)

        def process_dedigivolve(ctx):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_dedigivolve(target):
                removed = target.de_digivolve(1)
                if player.enemy:
                    player.enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_dedigivolve,
                filter_fn=lambda p: p.is_digimon and len(p.card_sources) > 1,
                is_optional=False
            )
        effect_dedigivolve.set_on_process_callback(process_dedigivolve)
        effects.append(effect_dedigivolve)

        # --- Inherited versions of all three effects ---

        inh_blocker = ICardEffect()
        inh_blocker.set_effect_name("BT23-077 Blocker (Inherited)")
        inh_blocker.set_effect_description("Blocker")
        inh_blocker._is_blocker = True
        inh_blocker.is_inherited_effect = True
        def cond_inh_blocker(context):
            return True
        inh_blocker.set_can_use_condition(cond_inh_blocker)
        effects.append(inh_blocker)

        inh_delete = ICardEffect()
        inh_delete.set_timing(EffectTiming.OnEnterFieldAnyone)
        inh_delete.set_effect_name("BT23-077 Delete cost <= 4 (Inherited)")
        inh_delete.set_effect_description(
            "[On Play] Delete 1 of your opponent's Digimon with a play cost of 4 or less."
        )
        inh_delete.is_on_play = True
        inh_delete.is_inherited_effect = True
        def cond_inh_delete(context):
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        inh_delete.set_can_use_condition(cond_inh_delete)
        # (process identical to non-inherited version)
        inh_delete.set_on_process_callback(process_delete)
        effects.append(inh_delete)

        inh_dedigivolve = ICardEffect()
        inh_dedigivolve.set_timing(EffectTiming.OnTappedAnyone)
        inh_dedigivolve.set_effect_name("BT23-077 De-Digivolve 1 (Inherited)")
        inh_dedigivolve.set_effect_description(
            "[All Turns] When this Digimon suspends, <De-Digivolve 1> "
            "1 of your opponent's Digimon."
        )
        inh_dedigivolve.is_inherited_effect = True
        def cond_inh_dedigivolve(context):
            if card and card.permanent_of_this_card() is None:
                return False
            ctx_perm = context.get('permanent')
            owner_perm = card.permanent_of_this_card()
            if owner_perm and ctx_perm and ctx_perm is not owner_perm:
                return False
            return True
        inh_dedigivolve.set_can_use_condition(cond_inh_dedigivolve)
        inh_dedigivolve.set_on_process_callback(process_dedigivolve)
        effects.append(inh_dedigivolve)

        return effects
```

### Example D: Modifier Registration + Keyword Grant + WhenRemoveField

BT24-040 Venusmon — Alt-digi with trait, cost reduction, keyword grants,
modifier-based protection, WhenRemoveField substitute effect.

```python
from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_040(CardScript):
    """BT24-040 Venusmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Alt-digi: Lv.5 with [TS] trait for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-040 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "TS"
        def condition0(context):
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Cost reduction: -5 if 3 or fewer security
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT24-040 Reduce play cost (5)")
        effect1.set_effect_description(
            "When this card would be played, if you have 3 or fewer "
            "security cards, reduce the play cost by 5."
        )
        effect1.cost_reduction = 5

        def condition1(context):
            if context.get('card_source') is not card:
                return False  # LEAK GUARD
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            return len(owner.security_cards) <= 3
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # On Play / When Digivolving: trash digi-cards, grant keyword, protection
        def make_main_effect(when_digivolving):
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone)
            eff.set_effect_name("BT24-040 Trash + keyword + protection")
            eff.set_effect_description("...")
            if when_digivolving:
                eff.is_when_digivolving = True
            else:
                eff.is_on_play = True
            eff._is_cannot_suspend_player = True

            def condition(context):
                if card and card.permanent_of_this_card() is None:
                    return False
                return True
            eff.set_can_use_condition(condition)

            def process(ctx):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if perm and not perm.has_no_digivolution_cards:
                    trashed = perm.trash_digivolution_cards(1)
                    if player:
                        player.trash_cards.extend(trashed)
                if perm:
                    perm.grant_keyword('_is_cannot_suspend_player')
                if perm and game:
                    from digimon_gym.engine.interfaces.modifiers import ModifierType
                    game.register_modifier(
                        ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                        value_fn=lambda: True, expiry='end_of_turn'
                    )
            eff.set_on_process_callback(process)
            return eff

        effects.append(make_main_effect(when_digivolving=False))
        effects.append(make_main_effect(when_digivolving=True))

        # [All Turns] [OPT] WhenRemoveField — substitute by placing to security
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.WhenRemoveField)
        effect5.set_effect_name("BT24-040 Substitute via security placement")
        effect5.set_effect_description(
            "[All Turns] [Once Per Turn] When any of your [TS] trait Digimon "
            "would leave the battle area, by placing 1 other Digimon with no "
            "digivolution cards as the bottom security card, they don't leave."
        )
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT24_040_AT")

        def condition5(context):
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)

        def process5(ctx):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_put_security(target_perm):
                player.put_permanent_to_security(target_perm)
            game.effect_select_own_permanent(
                player, on_put_security,
                filter_fn=lambda p: p.is_digimon,
                is_optional=True
            )
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn'
                )
        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
```

---

## Appendix: Selection Convention Ranges

These constants define which action IDs correspond to which selection targets:

| Range | Constant | Target |
|-------|----------|--------|
| 0–29 | `SEL_HAND_START` – `SEL_HAND_END` | Hand card by index |
| 30–39 | `SEL_REVEALED_START` – `SEL_REVEALED_END` | Revealed cards |
| 40–49 | `SEL_MY_SECURITY_START` – `SEL_MY_SECURITY_END` | Own security stack |
| 50–59 | `SEL_OPP_SECURITY_START` – `SEL_OPP_SECURITY_END` | Opponent's security |
| 99 | `SEL_MY_BREEDING` | Own breeding area |
| 100–113 | `SEL_MY_FIELD_START` – `SEL_MY_FIELD_END` | Own battle area |
| 114–127 | `SEL_OPP_FIELD_START` – `SEL_OPP_FIELD_END` | Opponent's battle area |
| 130–179 | `SEL_TRASH_START` – `SEL_TRASH_END` | Trash card by index |
| 1000–1009 | `SEL_EFFECT_CHOICE_START` – `SEL_EFFECT_CHOICE_END` | Effect branch choice |

Card scripts should NOT reference these directly — use `game.effect_select_*` methods which handle action mapping internally.
