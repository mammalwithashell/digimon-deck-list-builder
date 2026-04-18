from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_056(CardScript):
    """BT22-056 Guardromon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ----------------------------------------------------------------
        # Alternate digivolution requirement:
        #   Digivolve from Lv.3 [CS] Digimon for cost 2.
        # ----------------------------------------------------------------
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-056 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_color = CardColor.Black
        # Only [CS] trait — match C# PermanentCondition (Lv3 && HasCSTraits)
        effect0._alt_digi_trait = 'CS'

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ----------------------------------------------------------------
        # Shared process logic for [On Play] / [When Digivolving]:
        #   1. 1 of your opponent's Digimon gets -3000 DP for the turn.
        #   2. Then, if this Digimon's stack has 2 or more same-level cards,
        #      <De-Digivolve 1> 1 of your opponent's Digimon.
        # ----------------------------------------------------------------
        def _has_same_level_pair(perm) -> bool:
            """Mirror C# StackCards.GroupBy(Level).Any(g => g.Count() >= 2).

            StackCards = all card_sources except linked_cards.
            """
            if perm is None:
                return False
            linked = set(id(x) for x in getattr(perm, 'linked_cards', []))
            levels: Dict[int, int] = {}
            for cs in perm.card_sources:
                if id(cs) in linked:
                    continue
                lvl = getattr(cs, 'level', None)
                if lvl is None:
                    continue
                levels[lvl] = levels.get(lvl, 0) + 1
            return any(count >= 2 for count in levels.values())

        def _make_shared_process():
            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                perm = ctx.get('permanent')
                if not (player and game):
                    return
                # If the host permanent left the field somehow, bail.
                if perm is None or perm.top_card is None:
                    return

                # Step 1: Select 1 opponent Digimon and give it -3000 DP for the turn.
                def on_dp_target(target_perm):
                    if target_perm is None:
                        return
                    from ....interfaces.modifiers import ModifierType
                    # Target-equality guard prevents the -3000 DP from leaking
                    # to every permanent on the field (CHANGE_DP value_fn is
                    # called for every target; without this guard the modifier
                    # would apply to ALL Digimon).
                    game.register_modifier(
                        target_perm,
                        ModifierType.CHANGE_DP,
                        condition=(lambda t, c, _tp=target_perm: t is _tp),
                        value_fn=(lambda cur, t, c, _tp=target_perm:
                                  (cur - 3000) if t is _tp else cur),
                        expiry='end_of_turn',
                    )

                game.effect_select_opponent_permanent(
                    player,
                    on_dp_target,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt="Select 1 opponent's Digimon to give -3000 DP.",
                )

                # Step 2: If this Digimon's stack has 2+ same-level cards,
                # <De-Digivolve 1> 1 of your opponent's Digimon.
                if not _has_same_level_pair(perm):
                    return

                def on_de_digivolve(target_perm):
                    if target_perm is None:
                        return
                    removed = target_perm.de_digivolve(1)
                    enemy = player.enemy
                    if enemy and removed:
                        enemy.trash_cards.extend(removed)

                game.effect_select_opponent_permanent(
                    player,
                    on_de_digivolve,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt="Select 1 opponent's Digimon to de-digivolve.",
                )

            return process

        # ----------------------------------------------------------------
        # [On Play] instance
        # ----------------------------------------------------------------
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name(
            "BT22-056 Give 1 digimon -3k DP. if 2 or more same level in sources, De-Digivolve 1"
        )
        effect1.set_effect_description(
            "[On Play] 1 of your opponent's Digimon gets -3000 DP for the turn. "
            "Then, if this Digimon's stack has 2 or more same-level cards, "
            "<De-Digivolve 1> 1 of your opponent's Digimon. "
            "(Trash the top card. You can't trash past level 3 cards.)"
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Field presence — must be on the battle area to resolve.
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_make_shared_process())
        effects.append(effect1)

        # ----------------------------------------------------------------
        # [When Digivolving] instance (separate ICardEffect)
        # ----------------------------------------------------------------
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name(
            "BT22-056 Give 1 digimon -3k DP. if 2 or more same level in sources, De-Digivolve 1"
        )
        effect2.set_effect_description(
            "[When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the turn. "
            "Then, if this Digimon's stack has 2 or more same-level cards, "
            "<De-Digivolve 1> 1 of your opponent's Digimon. "
            "(Trash the top card. You can't trash past level 3 cards.)"
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_make_shared_process())
        effects.append(effect2)

        # ----------------------------------------------------------------
        # Inherited effect: [Opponent's Turn] This Digimon gets +2000 DP.
        # Applies only when BT22-056 is UNDER the top card (inherited)
        # AND it's the opponent's turn (from the host owner's perspective).
        # ----------------------------------------------------------------
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-056 [Opponent's Turn] +2000 DP")
        effect3.set_effect_description(
            "[Opponent's Turn] This Digimon gets +2000 DP."
        )
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 2000

        def condition3(context: Dict[str, Any]) -> bool:
            # Field presence — BT22-056 must be in some permanent's stack.
            if not card:
                return False
            host = card.permanent_of_this_card()
            if host is None:
                return False
            # "[Opponent's Turn]" — must NOT be the host owner's turn.
            owner = None
            if host.top_card and host.top_card.owner:
                owner = host.top_card.owner
            elif card.owner:
                owner = card.owner
            if owner is None:
                return False
            return not owner.is_my_turn
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
