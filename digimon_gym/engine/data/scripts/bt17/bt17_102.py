from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class _DynamicNamesList(list):
    """A list subclass that recomputes its contents on every access.

    C# ChangeCardNamesClass dynamically computes names in the CardNames
    property getter.  Our engine reads card.also_treated_as_names as a
    plain list.  This class bridges the gap: every time the list is
    iterated, checked for length, or used in ``in`` comparisons, it
    refreshes from the card's digivolution stack.
    """

    def __init__(self, card: 'CardSource'):
        super().__init__()
        self._card = card
        self._refreshing = False  # guard against recursion

    def _refresh(self):
        """Recompute names from Lv.3-and-lower digivolution cards."""
        if self._refreshing:
            return
        self._refreshing = True
        try:
            list.clear(self)
            card = self._card
            perm = card.permanent_of_this_card() if card else None
            if not perm or perm.top_card is not card:
                return
            seen: set = set()
            for digi_card in perm.card_sources:
                if digi_card is card:
                    continue
                level = getattr(digi_card, 'level', None)
                if level is not None and level <= 3:
                    # Read base card_names without triggering also_treated_as_names
                    base = digi_card.c_entity_base
                    if base:
                        name = base.card_name_eng
                        if name and name not in seen:
                            seen.add(name)
                            list.append(self, name)
        finally:
            self._refreshing = False

    def __iter__(self):
        self._refresh()
        return list.__iter__(self)

    def __len__(self):
        self._refresh()
        return list.__len__(self)

    def __bool__(self):
        self._refresh()
        return list.__len__(self) > 0

    def __contains__(self, item):
        self._refresh()
        return list.__contains__(self, item)

    def __add__(self, other):
        self._refresh()
        return list.__add__(self, list(other))

    def __radd__(self, other):
        self._refresh()
        return list.__add__(list(other), list(self))


class BT17_102(CardScript):
    """BT17-102 Greymon | Lv.4 White Digimon

    Alt digivolve: from Lv.3 [Agumon] for 2.
    [When Digivolving] If this Digimon's name is [Koromon], it gains +3000
        DP for the turn. Then, delete 1 of your opponent's Digimon with as
        much or less DP as this Digimon.
    [All Turns] This Digimon also has the names of all level 3 and lower
        cards in its digivolution cards.
    [On Deletion] You may play 1 Tamer card with [Tai Kamiya] or
        [Kari Kamiya] in its name from your hand without paying the cost or
        hatch in your breeding area.
    Inherited: Same as On Deletion.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Install dynamic name list for [All Turns] name absorption.
        # This replaces the static also_treated_as_names with a list that
        # recomputes from the digivolution stack on every access, mirroring
        # C#'s ChangeCardNamesClass dynamic evaluation in CardNames getter.
        card.also_treated_as_names = _DynamicNamesList(card)

        # --- Effect 0: Alt digivolve from Lv.3 [Agumon] for 2 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT17-102 Alt digivolve from Lv.3 Agumon")
        effect0.set_effect_description("Alt digivolve: from Lv.3 [Agumon] for 2")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Agumon"
        effect0._alt_digi_level = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] +3000 DP if named Koromon, then delete ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT17-102 When Digivolving: +3000 DP, delete")
        effect1.set_effect_description(
            "[When Digivolving] If this Digimon's name is [Koromon], +3000 DP "
            "for the turn. Then, delete 1 opponent Digimon with <= this DP."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            return perm.is_digimon
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # C# checks: card.PermanentOfThisCard().TopCard.EqualsCardName("Koromon")
            # The _DynamicNamesList on also_treated_as_names ensures names from
            # Lv.3-and-lower digivolution cards are included in card_names.
            if perm.contains_card_name("Koromon"):
                # +3000 DP for the turn (C#: ChangeDigimonDP, UntilEachTurnEnd)
                # change_dp appends to _dp_modifiers, cleared at end of turn.
                perm.change_dp(3000)

            # Then, delete 1 opponent Digimon with DP <= this Digimon's DP
            enemy = player.enemy
            if not enemy:
                return

            # C# evaluates DP dynamically in IsOpponentsDigimonSelectable
            # Use a closure that reads perm.dp at selection time (after +3000)
            def delete_filter(p):
                return p.is_digimon and p.dp is not None and p.dp <= perm.dp

            has_target = any(delete_filter(p) for p in enemy.battle_area)
            if has_target:
                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=delete_filter,
                    is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [All Turns] Has names of Lv.3 and lower digi-cards ---
        # C# uses ChangeCardNamesClass with a dynamic name function evaluated
        # inside the CardNames property getter.  We installed a _DynamicNamesList
        # on card.also_treated_as_names that recomputes on every access, so no
        # explicit process callback is needed.  This effect exists only as a
        # descriptive marker (no timing, no callback).
        effect2 = ICardEffect()
        effect2.set_effect_name("BT17-102 Has names of Lv.3- digi-cards")
        effect2.set_effect_description(
            "[All Turns] This Digimon also has the names of all level 3 and "
            "lower cards in its digivolution cards."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            if not perm.is_digimon:
                return False
            return perm.top_card is card
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Shared: On Deletion process (play tamer or hatch) ---
        def _on_deletion_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def tamer_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                return (c.contains_card_name('Tai Kamiya') or
                        c.contains_card_name('Kari Kamiya'))

            has_tamer = any(tamer_filter(c) for c in player.hand_cards)
            can_hatch = (player.breeding_area is None and
                         len(player.digitama_library_cards) > 0)

            if has_tamer and can_hatch:
                # Choose: play tamer or hatch
                def on_branch(choice: int):
                    if choice == 0:
                        game.effect_play_from_zone(
                            player, 'hand', tamer_filter,
                            free=True, is_optional=True)
                    else:
                        player.hatch()

                game.effect_choose_branch(
                    player, 2,
                    callback=on_branch,
                    branch_labels=["Play Tamer", "Hatch"])
            elif has_tamer:
                game.effect_play_from_zone(
                    player, 'hand', tamer_filter,
                    free=True, is_optional=True)
            elif can_hatch:
                player.hatch()

        # --- Effect 3: [On Deletion] Play tamer or hatch ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("BT17-102 On Deletion: play tamer or hatch")
        effect3.set_effect_description(
            "[On Deletion] Play 1 [Tai Kamiya]/[Kari Kamiya] Tamer from hand "
            "without paying cost, or hatch in breeding area."
        )
        effect3.is_on_deletion = True
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_on_deletion_process)
        effects.append(effect3)

        # --- Effect 4: Inherited [On Deletion] Same as main On Deletion ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("BT17-102 Inherited: On Deletion play tamer or hatch")
        effect4.set_effect_description(
            "[On Deletion] Play 1 [Tai Kamiya]/[Kari Kamiya] Tamer from hand "
            "without paying cost, or hatch in breeding area."
        )
        effect4.is_inherited_effect = True
        effect4.is_on_deletion = True
        effect4.is_optional = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_on_deletion_process)
        effects.append(effect4)

        return effects
