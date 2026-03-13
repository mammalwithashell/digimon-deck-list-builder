from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


def _has_x_antibody_in_sources(perm) -> bool:
    """Check if any digivolution card is named 'X Antibody'."""
    for cs in perm.card_sources:
        names = getattr(cs, 'card_names', []) or []
        if 'X Antibody' in names or 'XAntibody' in names:
            return True
    return False


class BT9_109(CardScript):
    """BT9-109 X Antibody | Option | White | Cost 1

    You may ignore this card's color requirements if you have a Digimon
        in play.

    [Security] Gain 1 memory, and add this card to its owner's hand.

    [Main] Place this card under 1 of your Digimon without [X Antibody]
        in its digivolution cards as its bottom digivolution card.

    Inherited:
    [All Turns] [X Antibody] cards in this Digimon's digivolution cards
        can't be removed by effects.
    [When Attacking] This Digimon can digivolve into a Digimon card with
        [X Antibody] in its traits in your hand for its digivolution cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Ignore color requirements if you have a Digimon in play ---
        # This is a static usage condition; not modeled as a game-changing effect
        # in the engine. The engine handles color requirements at play time.
        # We declare it for documentation fidelity.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT9-109 Ignore color requirements")
        effect0.set_effect_description(
            "You may ignore this card's color requirements if you have a "
            "Digimon in play."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            player = card.owner if card else None
            if not player:
                return False
            return any(p.is_digimon for p in player.battle_area)
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Security] Gain 1 memory, add this card to hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT9-109 Security: memory +1, add to hand")
        effect1.set_effect_description(
            "[Security] Gain 1 memory, and add this card to its owner's hand."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            # Gain 1 memory
            player.add_memory(1)
            # Add this card to hand
            if card in player.trash_cards:
                player.trash_cards.remove(card)
            player.hand_cards.append(card)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Main] Place under 1 of your Digimon without X Antibody ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name("BT9-109 Main: place under Digimon as digi-card")
        effect2.set_effect_description(
            "[Main] Place this card under 1 of your Digimon without "
            "[X Antibody] in its digivolution cards as its bottom "
            "digivolution card."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                if not p.is_digimon:
                    return False
                if p.owner != player:
                    return False
                # Must not have X Antibody in digivolution cards
                if _has_x_antibody_in_sources(p):
                    return False
                return True

            has_target = any(target_filter(p) for p in player.battle_area)
            if not has_target:
                return

            def on_select(target_perm):
                if target_perm and not target_perm.is_token:
                    target_perm.add_card_source_bottom(card)

            game.effect_select_own_permanent(
                player, on_select,
                filter_fn=target_filter,
                is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited [All Turns] X Antibody digi-cards can't be trashed ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.NoTiming)
        effect3.set_effect_name("BT9-109 Inherited: protect X Antibody digi-cards")
        effect3.set_effect_description(
            "[All Turns] [X Antibody] cards in this Digimon's digivolution "
            "cards can't be removed by effects."
        )
        effect3.is_inherited_effect = True
        effect3.is_declarative = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        # Register CANNOT_TRASH_DIGIVOLUTION_CARDS modifier on the permanent
        def process3(ctx: Dict[str, Any]):
            game = ctx.get('game')
            if not game:
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            from ....interfaces.modifiers import ModifierType
            game.register_modifier(
                perm, ModifierType.CANNOT_TRASH_DIGIVOLUTION_CARDS,
                condition=lambda p, c: (
                    'X Antibody' in (getattr(c, 'card_names', []) or []) or
                    'XAntibody' in (getattr(c, 'card_names', []) or [])
                ),
                expiry='permanent')
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: Inherited [When Attacking] Digivolve into X Antibody from hand ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnAllyAttack)
        effect4.set_effect_name("BT9-109 Inherited: digivolve into X Antibody on attack")
        effect4.set_effect_description(
            "[When Attacking] This Digimon can digivolve into a Digimon card "
            "with [X Antibody] in its traits in your hand for its digivolution "
            "cost."
        )
        effect4.is_inherited_effect = True
        effect4.is_optional = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            return len(player.hand_cards) >= 1
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            def x_antibody_digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('X Antibody' in t or 'X-Antibody' in t for t in traits):
                    return False
                return True

            has_target = any(x_antibody_digi_filter(c) for c in player.hand_cards)
            if has_target:
                game.effect_digivolve_from_hand(
                    player, perm, x_antibody_digi_filter,
                    is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
