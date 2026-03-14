from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_013(CardScript):
    """BT23-013 Jesmon | Lv.6 | Red | Royal Knight, CS

    Alt-digi: from [SaviorHuckmon] or Lv.5 with [CS] trait for cost 3.
    Alt-digi: from [Huckmon] for cost 5 (if opponent has a Digimon with 10000+ DP).
    <Rush> <Alliance>
    [When Digivolving] [When Attacking] You may play 1 [Atho, Rene & Por] Token
        or, from your hand or trash, 1 Digimon card with [Sistermon] in its name
        without paying the cost. This effect can't play cards with the same names
        as any of your Digimon.
    [Your Turn] [Once Per Turn] When any of your other Digimon are played,
        this Digimon may attack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt-digi from [SaviorHuckmon] or Lv.5 [CS] for cost 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-013 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution: from [SaviorHuckmon] or Lv.5 [CS] for cost 3")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not permanent or not permanent.top_card:
                return False
            top = permanent.top_card
            # Accept SaviorHuckmon (any level)
            if top.contains_card_name('SaviorHuckmon'):
                return True
            # Accept Lv.5 with [CS] trait
            level = getattr(top, 'level', None)
            if level == 5:
                traits = getattr(top, 'card_traits', []) or []
                if any('CS' in t for t in traits):
                    return True
            return False
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Alt-digi from [Huckmon] for cost 5 (if opp has 10000+ DP Digimon) ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-013 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution: from [Huckmon] for cost 5 (opp has 10000+ DP)")
        effect1._alt_digi_cost = 5
        effect1._alt_digi_name = "Huckmon"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.contains_card_name('Huckmon')):
                return False
            # Condition: opponent has a Digimon with 10000+ DP
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            return any(
                p.is_digimon and p.dp is not None and p.dp >= 10000
                for p in enemy.battle_area
            )
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Rush ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-013 Rush")
        effect2.set_effect_description("Rush")
        effect2._is_rush = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: Alliance ---
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-013 Alliance")
        effect3.set_effect_description("Alliance")
        effect3.is_on_attack = True
        effect3._is_alliance = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # --- Shared play process for WD/WA ---
        def _play_sistermon_or_token(ctx: Dict[str, Any]):
            """Play 1 Atho/Rene/Por Token OR 1 Sistermon from hand/trash (no duplicate names)."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Get names of all Digimon currently on the field
            field_names = set()
            for p in player.battle_area:
                if p.top_card:
                    for n in p.top_card.card_names:
                        field_names.add(n.lower())

            def play_filter(c):
                if not c.is_digimon:
                    return False
                if not c.contains_card_name('Sistermon'):
                    return False
                for n in c.card_names:
                    if n.lower() in field_names:
                        return False
                return True

            # Check availability
            has_token_on_field = any(
                p.contains_card_name('Atho') for p in player.battle_area
            )
            has_sistermon = any(
                play_filter(c) for c in player.hand_cards
            ) or any(
                play_filter(c) for c in player.trash_cards
            )

            can_play_token = not has_token_on_field
            can_play_sistermon = has_sistermon

            if can_play_token and can_play_sistermon:
                # Both available - let agent choose
                def on_branch(choice: int):
                    if choice == 0:
                        game.effect_play_token(player, 'atho_rene_por')
                    else:
                        game.effect_play_from_zone(
                            player, 'hand_or_trash', play_filter, free=True, is_optional=True)
                game.effect_choose_branch(
                    player, 2, on_branch,
                    branch_labels=["Play Atho/Rene/Por Token", "Play Sistermon"])
            elif can_play_sistermon:
                game.effect_play_from_zone(
                    player, 'hand_or_trash', play_filter, free=True, is_optional=True)
            elif can_play_token:
                game.effect_play_token(player, 'atho_rene_por')

        # --- Effect 4: [When Digivolving] Play token or Sistermon ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT23-013 WD: Play Token or Sistermon")
        effect4.set_effect_description(
            "[When Digivolving] You may play 1 [Atho, Rene & Por] Token or, "
            "from your hand or trash, 1 Digimon card with [Sistermon] in its "
            "name without paying the cost."
        )
        effect4.is_when_digivolving = True
        effect4.is_optional = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_play_sistermon_or_token)
        effects.append(effect4)

        # --- Effect 5: [When Attacking] Play token or Sistermon ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
        effect5.set_effect_name("BT23-013 WA: Play Token or Sistermon")
        effect5.set_effect_description(
            "[When Attacking] You may play 1 [Atho, Rene & Por] Token or, "
            "from your hand or trash, 1 Digimon card with [Sistermon] in its "
            "name without paying the cost."
        )
        effect5.is_on_attack = True
        effect5.is_optional = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be THIS Digimon attacking
            perm = card.permanent_of_this_card()
            ctx_perm = context.get('attacker') or context.get('permanent')
            if perm and ctx_perm and ctx_perm is not perm:
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(_play_sistermon_or_token)
        effects.append(effect5)

        # --- Effect 6: [Your Turn][OPT] When other Digimon played, this may attack ---
        # Uses _is_play_observer to observe plays of OTHER Digimon
        effect6 = ICardEffect()
        effect6.set_effect_name("BT23-013 This Digimon may attack on ally play")
        effect6.set_effect_description(
            "[Your Turn] [Once Per Turn] When any of your other Digimon are "
            "played, this Digimon may attack."
        )
        effect6.is_optional = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("BT23_013_YT")
        effect6._is_play_observer = True

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # The played permanent must be a Digimon (and is already guaranteed
            # to be a different permanent by _is_play_observer)
            played_perm = context.get('played_permanent')
            if not played_perm or not played_perm.is_digimon:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """This Digimon may attack - unsuspend it."""
            perm = card.permanent_of_this_card() if card else None
            if perm and perm.is_suspended:
                perm.unsuspend()

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
