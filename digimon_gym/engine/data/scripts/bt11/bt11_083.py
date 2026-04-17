from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_083(CardScript):
    """BT11-083 LadyDevimon | Lv.5

    [When Digivolving] You may trash 1 card in your hand. If you do,
    you may return 1 [Mirei Mikagura] or 1 card with the [Angel],
    [Archangel] or [Fallen Angel] trait from your trash to your hand.

    [Your Turn] [Once Per Turn] When you play [Angewomon] or
    [Mirei Mikagura], gain 1 memory.

    [Inherited] [Opponent's Turn] While you have a yellow Digimon in
    play, all of your Digimon with the [Angel], [Archangel] or
    [Fallen Angel] trait gain <Retaliation>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_angel_trait_card(c) -> bool:
            """True if CardSource has Angel/Archangel/Fallen Angel trait."""
            traits = getattr(c, 'card_traits', []) or []
            for trait in traits:
                if not trait:
                    continue
                if (
                    'Angel' in trait
                    or 'Archangel' in trait
                    or 'Fallen Angel' in trait
                    or 'FallenAngel' in trait
                ):
                    return True
            return False

        def _is_valid_return_target(c) -> bool:
            """Card in trash qualifies: Mirei Mikagura name OR Angel-family trait."""
            if c.contains_card_name('Mirei Mikagura'):
                return True
            if c.contains_card_name('MireiMikagura'):
                return True
            return _is_angel_trait_card(c)

        def _has_yellow_digimon_in_play(owner) -> bool:
            if not owner:
                return False
            # Import locally to avoid top-level cycles.
            from ....data.enums import CardColor
            for p in owner.battle_area:
                if not p.is_digimon:
                    continue
                top = p.top_card
                if top is None:
                    continue
                colors = getattr(top, 'card_colors', None) or []
                if CardColor.Yellow in colors:
                    return True
            return False

        # ── Effect 0: [When Digivolving] optional discard + optional
        # return-from-trash. Both player choices are exposed sequentially
        # via nested selection callbacks so the RL agent has full agency.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name(
            "BT11-083 Trash 1 card from hand, then return 1 Mirei/Angel-family "
            "card from trash to hand"
        )
        effect0.set_effect_description(
            "[When Digivolving] You may trash 1 card in your hand. If you do, "
            "you may return 1 [Mirei Mikagura] or 1 card with the [Angel], "
            "[Archangel] or [Fallen Angel] trait from your trash to your hand."
        )
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Optional trash from hand; on confirmed discard, optionally
            return an eligible card from trash to hand (player chooses)."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.hand_cards:
                return

            def _offer_return_from_trash():
                """Second step — runs only after a successful discard."""
                # Build the list of eligible trash cards and expose them to
                # the agent via SelectTrash. Never auto-select.
                from ....data.enums import GamePhase
                from ....game.constants import SEL_TRASH_START

                valid_trash = [
                    SEL_TRASH_START + i
                    for i, c in enumerate(player.trash_cards)
                    if _is_valid_return_target(c)
                ]
                if not valid_trash:
                    return

                def on_trash_selected(action_id: int):
                    idx = action_id - SEL_TRASH_START
                    if not (0 <= idx < len(player.trash_cards)):
                        return
                    chosen = player.trash_cards[idx]
                    # Double-check filter in case state changed.
                    if not _is_valid_return_target(chosen):
                        return
                    player.trash_cards.remove(chosen)
                    player.hand_cards.append(chosen)

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_selected,
                    valid_trash, is_optional=True,
                    prompt=(
                        "Select 1 [Mirei Mikagura] or [Angel]/[Archangel]/"
                        "[Fallen Angel] card from your trash to return to hand."
                    ),
                )

            def hand_filter(c):
                return True

            def on_trashed(selected):
                # Selection callback — only fires when the player chose a
                # card to discard. Gate the return-from-trash on an
                # actually-discarded card ("If you do, ...").
                if selected not in player.hand_cards:
                    return
                player.hand_cards.remove(selected)
                player.trash_cards.append(selected)
                _offer_return_from_trash()

            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True,
                prompt="Select 1 card in your hand to trash.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ── Effect 1: [Your Turn][Once Per Turn] When you play [Angewomon]
        # or [Mirei Mikagura], gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT11-083 Memory +1")
        effect1.set_effect_description(
            "[Your Turn][Once Per Turn] When you play [Angewomon] or "
            "[Mirei Mikagura], gain 1 memory."
        )
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT11_083")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if context.get('event_player') is not card.owner:
                return False
            played_card = context.get('played_card')
            if not played_card:
                return False
            return (
                played_card.contains_card_name('Angewomon')
                or played_card.contains_card_name('Mirei Mikagura')
                or played_card.contains_card_name('MireiMikagura')
            )

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Effect 2: [Inherited] [Opponent's Turn] While you have a yellow
        # Digimon in play, all of your Digimon with the [Angel], [Archangel]
        # or [Fallen Angel] trait gain <Retaliation>.
        #
        # Implemented as an aura-style keyword grant via
        # `_applies_to_all_own_digimon` + `_keyword_permanent_condition`, so
        # `permanent.has_keyword('_is_retaliation')` scans this effect on
        # other friendly permanents and checks the Angel-trait filter.
        def _angel_family_perm_filter(target_perm) -> bool:
            top = target_perm.top_card
            if top is None:
                return False
            return _is_angel_trait_card(top)

        effect2 = ICardEffect()
        effect2.set_effect_name(
            "BT11-083 [Inherited] Retaliation aura for Angel-family Digimon"
        )
        effect2.set_effect_description(
            "[Opponent's Turn] While you have a yellow Digimon in play, all "
            "of your Digimon with the [Angel], [Archangel] or [Fallen Angel] "
            "trait gain <Retaliation>."
        )
        effect2.is_inherited_effect = True
        effect2._is_retaliation = True
        effect2._applies_to_all_own_digimon = True
        effect2._keyword_permanent_condition = _angel_family_perm_filter

        def condition2(context: Dict[str, Any]) -> bool:
            # Must still be on the field.
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if owner is None:
                return False
            # [Opponent's Turn] gate.
            if owner.is_my_turn:
                return False
            # "While you have a yellow Digimon in play" gate.
            if not _has_yellow_digimon_in_play(owner):
                return False
            # The self-inherit path of Permanent.has_keyword() does NOT
            # apply `_keyword_permanent_condition` — it checks the condition
            # with `ctx['permanent'] = self` (the permanent being scanned),
            # then returns True unconditionally. Gate the condition here
            # on the scanned permanent's top-card trait so non-Angel hosts
            # (and any sibling scans that pass a scanned permanent) are
            # properly excluded.
            scanned = context.get('permanent')
            if scanned is not None:
                top = getattr(scanned, 'top_card', None)
                if top is None or not _is_angel_trait_card(top):
                    return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
