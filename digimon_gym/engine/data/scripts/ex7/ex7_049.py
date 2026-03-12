from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_049(CardScript):
    """EX7-049 Metallicdramon | Lv.6 Black Sky Dragon Digimon

    [On Play] [When Attacking] <De-Digivolve 4> 1 of your opponent's Digimon.
    [When Digivolving] None of your opponent's level 4 or lower Digimon can
        digivolve until the end of their turn.
    [All Turns] [Once Per Turn] When this Digimon would leave battle area other
        than by one of your effects, you may play 1 Digimon card with the
        [Rock Dragon] or [Earth Dragon] trait from your trash without paying
        the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] <De-Digivolve 4> 1 opponent Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX7-049 <De-Digivolve 4> on play")
        effect0.set_effect_description(
            "[On Play] <De-Digivolve 4> 1 of your opponent's Digimon."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(4)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon, is_optional=False)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [When Attacking] <De-Digivolve 4> 1 opponent Digimon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("EX7-049 <De-Digivolve 4> when attacking")
        effect1.set_effect_description(
            "[When Attacking] <De-Digivolve 4> 1 of your opponent's Digimon."
        )
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(4)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon, is_optional=False)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] Opponent Lv.4 or lower can't digivolve ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX7-049 Opponent Lv.4 or lower can't digivolve")
        effect2.set_effect_description(
            "[When Digivolving] None of your opponent's level 4 or lower "
            "Digimon can digivolve until the end of their turn."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....interfaces.modifiers import ModifierType
            enemy = player.enemy
            if not enemy:
                return
            for field_perm in enemy.battle_area:
                lvl = getattr(field_perm, 'level', None)
                if lvl is not None and lvl <= 4:
                    game.register_modifier(
                        field_perm,
                        ModifierType.CANNOT_DIGIVOLVE,
                        condition=lambda p, c, fp=field_perm: p is fp,
                        expiry='end_of_opponent_turn',
                    )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [All Turns] [Once Per Turn] When this Digimon would leave, play from trash ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("EX7-049 Play Rock Dragon/Earth Dragon from trash")
        effect3.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon would leave battle "
            "area other than by one of your effects, you may play 1 Digimon "
            "card with the [Rock Dragon] or [Earth Dragon] trait from your "
            "trash without paying the cost."
        )
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX7_049_WhenLeave")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Check that trash has a qualifying Digimon
            return any(
                getattr(c, 'is_digimon', False) and
                any('Rock Dragon' in t or 'Earth Dragon' in t
                    for t in (getattr(c, 'card_traits', []) or []))
                for c in owner.trash_cards
            )
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Rock Dragon' in t or 'Earth Dragon' in t for t in traits)
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
