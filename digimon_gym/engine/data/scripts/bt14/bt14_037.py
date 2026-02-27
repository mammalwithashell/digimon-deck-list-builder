from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_037(CardScript):
    """BT14-037 MagnaAngemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-037 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def has_five_or_fewer_security(player: Any) -> bool:
            if player is None:
                return False
            sec = getattr(player, 'security', None)
            if sec is not None:
                try:
                    return len(sec) <= 5
                except Exception:
                    pass
            get_security_count = getattr(player, 'get_security_count', None)
            if callable(get_security_count):
                try:
                    return int(get_security_count()) <= 5
                except Exception:
                    return False
            return False

        def get_security_count(player: Any) -> int:
            if player is None:
                return 0
            sec = getattr(player, 'security', None)
            if sec is not None:
                try:
                    return len(sec)
                except Exception:
                    pass
            fn = getattr(player, 'get_security_count', None)
            if callable(fn):
                try:
                    return int(fn())
                except Exception:
                    return 0
            return 0

        def apply_dp_minus(ctx: Dict[str, Any], amount: int):
            if amount <= 0:
                return
            game = ctx.get('game')
            opponent = ctx.get('opponent')
            if game is None or opponent is None:
                return

            candidates = []
            get_battle = getattr(opponent, 'get_battle_area', None)
            if callable(get_battle):
                try:
                    area = get_battle()
                    candidates = list(area) if area is not None else []
                except Exception:
                    candidates = []
            if not candidates:
                field = getattr(opponent, 'battle_area', None)
                if field is not None:
                    try:
                        candidates = list(field)
                    except Exception:
                        candidates = []
            if not candidates:
                return

            target = candidates[0]

            add_dp = getattr(game, 'add_dp_change_until_end_of_turn', None)
            if callable(add_dp):
                try:
                    add_dp(target, -amount, source=ctx.get('permanent'))
                    return
                except TypeError:
                    try:
                        add_dp(target, -amount)
                        return
                    except Exception:
                        pass
                except Exception:
                    pass

            grant_dp = getattr(game, 'grant_dp_mod_until_end_of_turn', None)
            if callable(grant_dp):
                try:
                    grant_dp(target, -amount, source=ctx.get('permanent'))
                    return
                except TypeError:
                    try:
                        grant_dp(target, -amount)
                        return
                    except Exception:
                        pass
                except Exception:
                    pass

            if hasattr(target, 'dp'):
                try:
                    target.dp -= amount
                except Exception:
                    pass

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn,
        # 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-037 Recovery +1 and opponent's 1 Digimon reduces DP")
        effect1.set_effect_description("[On Play] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn, 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            return has_five_or_fewer_security(player)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.recovery(1)
                sec_count = get_security_count(player)
                apply_dp_minus(ctx, sec_count * 1000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn,
        # 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-037 Recovery +1 and opponent's 1 Digimon reduces DP")
        effect2.set_effect_description("[When Digivolving] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn, 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            return has_five_or_fewer_security(player)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.recovery(1)
                sec_count = get_security_count(player)
                apply_dp_minus(ctx, sec_count * 1000)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
