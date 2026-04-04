from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_034(CardScript):
    """BT24-034 Aegiomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: named [Elecmon] OR Lv.3 with [TS] trait, cost 2
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-034 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Elecmon"
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent is None:
                return False
            top = getattr(permanent, 'top_card', None)
            if top is None:
                return False
            has_name = any('Elecmon' in n for n in getattr(top, 'card_names', []))
            has_ts = any('TS' in t for t in getattr(top, 'card_traits', []))
            is_lv3 = getattr(top, 'level', 0) == 3
            return has_name or (is_lv3 and has_ts)
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: barrier (own)
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-034 Barrier")
        effect1.set_effect_description("Barrier")
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Shared process for When Moving / On Play / When Digivolving:
        # "By adding your top security card to the hand, you may play 1 [TS] Tamer
        # from your hand without paying the cost."
        # C# flow: activation requires security > 0 (checked in condition).
        # On activation: security → hand (mandatory cost), then optionally play tamer.
        def make_shared_process():
            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                # Pay cost: move top security to hand (mandatory upon activation)
                if not player.security_cards:
                    return
                top_sec = player.security_cards.pop(0)
                player.hand_cards.append(top_sec)

                # Build tamer filter (checked AFTER security moved to hand)
                field_perms = player.battle_area if hasattr(player, 'battle_area') else []
                field_tamer_names = set()
                for fp in field_perms:
                    if getattr(fp, 'is_tamer', False):
                        top = getattr(fp, 'top_card', None)
                        if top:
                            for n in getattr(top, 'card_names', []):
                                field_tamer_names.add(n.lower())

                def tamer_filter(c):
                    if not getattr(c, 'is_tamer', False):
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    if not any('TS' in t for t in traits):
                        return False
                    card_names = getattr(c, 'card_names', []) or []
                    for n in card_names:
                        if n.lower() in field_tamer_names:
                            return False
                    return True

                # "you may play 1 [TS] Tamer" — optional (C# canNoSelect: true)
                if any(tamer_filter(c) for c in player.hand_cards):
                    game.effect_play_from_zone(
                        player, 'hand',
                        filter_fn=tamer_filter,
                        free=True,
                        is_optional=True
                    )

            return process

        # Timing: EffectTiming.OnMove — When Moving
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnMove)
        effect2.set_effect_name("BT24-034 Add top security to hand, play [TS] Tamer free")
        effect2.set_effect_description("[When Moving] By adding your top security card to the hand, you may play 1 [TS] trait Tamer card from your hand without paying the cost. This effect can't play cards with the same name as any of your Tamers.")
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be this Digimon moving (engine passes "moved_permanent" in OnMove context)
            moved_perm = context.get('moved_permanent') if context else None
            my_perm = card.permanent_of_this_card() if card else None
            if moved_perm is not None and my_perm is not None:
                if moved_perm is not my_perm:
                    return False
            player = card.owner if card else None
            if player and not getattr(player, 'security_cards', None):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(make_shared_process())
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone — On Play
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-034 Add top security to hand, play [TS] Tamer free")
        effect3.set_effect_description("[On Play] By adding your top security card to the hand, you may play 1 [TS] trait Tamer card from your hand without paying the cost. This effect can't play cards with the same name as any of your Tamers.")
        effect3.is_on_play = True
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if player and not getattr(player, 'security_cards', None):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(make_shared_process())
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone — When Digivolving
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-034 Add top security to hand, play [TS] Tamer free")
        effect4.set_effect_description("[When Digivolving] By adding your top security card to the hand, you may play 1 [TS] trait Tamer card from your hand without paying the cost. This effect can't play cards with the same name as any of your Tamers.")
        effect4.is_when_digivolving = True
        effect4.is_optional = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if player and not getattr(player, 'security_cards', None):
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(make_shared_process())
        effects.append(effect4)

        # Factory effect: barrier (inherited / ESS)
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-034 Barrier")
        effect5.set_effect_description("Barrier")
        effect5.is_inherited_effect = True
        effect5._is_barrier = True

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
