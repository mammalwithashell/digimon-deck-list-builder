from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST20_05(CardScript):
    """ST20-05 Gatomon | Lv.4 Yellow Digimon | Holy Beast/ADVENTURE | DP 4000 | Cost 4

    Alt Digivolution: Lv.3 w/[ADVENTURE] trait for cost 2 (color-agnostic).

    [Security] At the end of the battle, play this card without paying the cost.
    [On Play] [When Digivolving] Give 2 of your opponent's Digimon
        <Security A. -1> (This Digimon checks 1 fewer security card.)
        until their turn ends.
    Inherited Effect:
    [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets
        -2000 DP for the turn.

    C# ref: DCGO/Assets/Scripts/CardEffect/ST20/Yellow/ST20_05.cs
    - AddSelfDigivolutionRequirementStaticEffect(HasAdventureTraits, Lv=3, cost=2)
    - PlaySelfDigimonAfterBattleSecurityEffect (SecuritySkill)
    - SelectPermanent(max=2) + ChangeDigimonSAttack(-1, UntilOpponentTurnEnd)
      — shared for [On Play] and [When Digivolving]
    - [When Attacking] OPT + SelectPermanent(max=1) + ChangeDigimonDP(-2000, UntilEachTurnEnd)
      hash "DP-2000_ST20-005"
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ──────────────────────────────────────────────────────────────
        # Effect 0: Alt-digi — Lv.3 w/[ADVENTURE] trait for cost 2
        # ──────────────────────────────────────────────────────────────
        effect0 = ICardEffect()
        effect0.set_effect_name("ST20-05 Alt digi: ADVENTURE Lv.3 cost 2")
        effect0.set_effect_description(
            "Digivolve: Lv.3 w/[ADVENTURE] trait for cost 2."
        )
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_trait = "ADVENTURE"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ──────────────────────────────────────────────────────────────
        # Effect 1: [Security] At the end of the battle, play this card
        # ──────────────────────────────────────────────────────────────
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("ST20-05 Security: play this card after battle")
        effect1.set_effect_description(
            "[Security] At the end of the battle, play this card without "
            "paying the cost."
        )
        effect1.is_security_effect = True
        effect1._security_play_self = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            game.effect_play_from_security(player, card)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ──────────────────────────────────────────────────────────────
        # Shared helper: pick up to 2 opponent Digimon, apply SA-1
        # until opponent's turn ends.
        # ──────────────────────────────────────────────────────────────
        def _apply_sa_minus_1_to_two_opponent_digimon(ctx: Dict[str, Any]):
            from ....interfaces.modifiers import ModifierType

            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if enemy is None:
                return

            selected: List[Any] = []

            def make_filter():
                def _filter(p):
                    if not p.is_digimon:
                        return False
                    # Exclude already-selected permanents from the second pick
                    if any(p is s for s in selected):
                        return False
                    return True
                return _filter

            def register_sa_minus_1(target_perm):
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_SECURITY_ATTACK,
                    condition=lambda t, c, _tp=target_perm: t is _tp,
                    value_fn=lambda cur, t, c: cur - 1,
                    expiry='end_of_opponent_turn',
                )

            def pick_second(first_target):
                selected.append(first_target)
                register_sa_minus_1(first_target)
                # Attempt the second selection; skips if no more valid targets
                game.effect_select_opponent_permanent(
                    player, pick_second_callback,
                    filter_fn=make_filter(),
                    is_optional=False,
                    prompt="Select another opponent's Digimon to give Security Attack -1.",
                )

            def pick_second_callback(second_target):
                selected.append(second_target)
                register_sa_minus_1(second_target)

            game.effect_select_opponent_permanent(
                player, pick_second,
                filter_fn=make_filter(),
                is_optional=False,
                prompt="Select an opponent's Digimon to give Security Attack -1 (1 of 2).",
            )

        def _shared_on_play_wd_condition(context: Dict[str, Any]) -> bool:
            # Must be on field
            if card and card.permanent_of_this_card() is None:
                return False
            # Must have an opponent Digimon to target
            owner = card.owner if card else None
            if owner is None:
                return False
            enemy = owner.enemy
            if enemy is None:
                return False
            return any(p.is_digimon for p in enemy.battle_area)

        # ──────────────────────────────────────────────────────────────
        # Effect 2: [On Play] Give 2 opponent Digimon <Security A. -1>
        # ──────────────────────────────────────────────────────────────
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("ST20-05 On Play: 2 opponent Digimon Security A. -1")
        effect2.set_effect_description(
            "[On Play] Give 2 of your opponent's Digimon <Security A. -1> "
            "(This Digimon checks 1 fewer security card.) until their turn ends."
        )
        effect2.is_on_play = True
        effect2.set_can_use_condition(_shared_on_play_wd_condition)
        effect2.set_on_process_callback(_apply_sa_minus_1_to_two_opponent_digimon)
        effects.append(effect2)

        # ──────────────────────────────────────────────────────────────
        # Effect 3: [When Digivolving] Give 2 opponent Digimon <Security A. -1>
        # ──────────────────────────────────────────────────────────────
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("ST20-05 When Digivolving: 2 opponent Digimon Security A. -1")
        effect3.set_effect_description(
            "[When Digivolving] Give 2 of your opponent's Digimon <Security A. -1> "
            "(This Digimon checks 1 fewer security card.) until their turn ends."
        )
        effect3.is_when_digivolving = True
        effect3.set_can_use_condition(_shared_on_play_wd_condition)
        effect3.set_on_process_callback(_apply_sa_minus_1_to_two_opponent_digimon)
        effects.append(effect3)

        # ──────────────────────────────────────────────────────────────
        # Effect 4: Inherited [When Attacking] [OPT] -2000 DP
        # ──────────────────────────────────────────────────────────────
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("ST20-05 Inherited [When Attacking]: -2000 DP")
        effect4.set_effect_description(
            "[When Attacking] [Once Per Turn] 1 of your opponent's Digimon "
            "gets -2000 DP for the turn."
        )
        effect4.is_inherited_effect = True
        effect4.is_on_attack = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("DP-2000_ST20-005")

        def condition4(context: Dict[str, Any]) -> bool:
            # Must be on field
            if card and card.permanent_of_this_card() is None:
                return False
            host_perm = card.permanent_of_this_card() if card else None
            # THIS Digimon must be the attacker. `attacker` is the actual
            # attacking permanent (set by combat.execute_effects).
            attacker = context.get('attacker')
            if host_perm is None or attacker is None or attacker is not host_perm:
                return False
            # Must be effect owner's turn
            owner = card.owner if card else None
            if owner is None or not owner.is_my_turn:
                return False
            # Must have an opponent Digimon to target
            enemy = owner.enemy
            if enemy is None:
                return False
            return any(p.is_digimon for p in enemy.battle_area)

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            from ....interfaces.modifiers import ModifierType

            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if enemy is None:
                return

            def on_dp_target(target_perm):
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_DP,
                    condition=lambda t, c, _tp=target_perm: t is _tp,
                    value_fn=lambda cur, t, c: cur - 2000,
                    expiry='end_of_turn',
                )

            game.effect_select_opponent_permanent(
                player, on_dp_target,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 opponent's Digimon to get -2000 DP this turn.",
            )

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
