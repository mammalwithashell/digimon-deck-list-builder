from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_037(CardScript):
    """BT23-037 Tentomon | Lv.3 Green/Yellow Insectoid/Hudie/CS

    Alt Digivolve: Lv.2 w/[CS] trait for cost 0.
    [Your Turn] When this Digimon would digivolve into a Digimon card with the
        [CS] trait, reduce the digivolution cost by 1.
    Inherited:
    [When Attacking] [Once Per Turn] You may play 1 play cost 5 or lower
        Digimon card with the [Hudie] trait from your hand without paying the
        cost. The Digimon this effect played can't digivolve and is deleted
        at the end of your opponent's turn.

    C# ref: DCGO/Assets/Scripts/CardEffect/BT23/Green/BT23_037.cs
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt Digivolve — Lv.2 w/[CS] trait for cost 0 ---
        # C# ref: AddSelfDigivolutionRequirementStaticEffect with
        #   IsLevel2 && HasCSTraits
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-037 Alt digivolve Lv.2 w/[CS] cost 0")
        effect0.set_effect_description("[Digivolve] Lv.2 w/[CS] trait: Cost 0")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "CS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] Cost reduction for CS-trait digivolve ---
        # NON-INHERITED (isInheritedEffect: false in C# — applies only when
        # Tentomon itself is the one digivolving, i.e. Tentomon is top card).
        # Engine picks this up in Player.digivolve() which scans
        # source.effect_list(WhenWouldDigivolve) on the permanent's stack.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenWouldDigivolve)
        effect1.set_effect_name("BT23-037 Reduce digivolution cost by 1 for [CS]")
        effect1.set_effect_description(
            "[Your Turn] When this Digimon would digivolve into a Digimon card "
            "with the [CS] trait, reduce the digivolution cost by 1."
        )
        effect1.cost_reduction = 1
        # Non-inherited: only active on the top card's copy.

        def condition1(context: Dict[str, Any]) -> bool:
            # Field presence
            if card is None or card.permanent_of_this_card() is None:
                return False
            # [Your Turn] restriction
            owner = card.owner
            if owner is None or not owner.is_my_turn:
                return False
            # "This Digimon would digivolve": only the host permanent
            host_perm = card.permanent_of_this_card()
            ctx_perm = context.get('permanent')
            if ctx_perm is not None and ctx_perm is not host_perm:
                return False
            # Target must have [CS] trait (exact membership, not substring).
            incoming = context.get('card_source')
            if incoming is None:
                return False
            traits = getattr(incoming, 'card_traits', []) or []
            if 'CS' not in traits:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Inherited [When Attacking] [OPT] play Hudie ≤5 free ---
        # The played Digimon can't digivolve and is deleted at end of
        # opponent's turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name(
            "BT23-037 [WA][OPT] Play 1 [Hudie] Digimon ≤5 from hand free"
        )
        effect2.set_effect_description(
            "[When Attacking] [Once Per Turn] You may play 1 play cost 5 or "
            "lower Digimon card with the [Hudie] trait from your hand without "
            "paying the cost. The Digimon this effect played can't digivolve "
            "and is deleted at the end of your opponent's turn."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.is_on_attack = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT23_037_WA")

        def condition2(context: Dict[str, Any]) -> bool:
            # Field presence
            if card is None or card.permanent_of_this_card() is None:
                return False
            # Inherited effect host must be the attacker
            host_perm = card.permanent_of_this_card()
            attacker = context.get('attacker')
            if attacker is not None and attacker is not host_perm:
                return False
            # Owner gate — only fires on owner's attack
            owner = card.owner
            event_player = context.get('event_player') or context.get('player')
            if owner is not None and event_player is not None and event_player is not owner:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play a ≤5-cost [Hudie] Digimon from hand for free, then apply
            CANNOT_DIGIVOLVE + delete-at-end-of-opponent-turn to the played
            permanent.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if 'Hudie' not in traits:
                    return False
                return True

            def on_played(selected_card, played_perm):
                """After the Hudie Digimon is played, apply restrictions."""
                if played_perm is None:
                    return

                # Restriction 1: Can't digivolve (until deleted at EoOT)
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    played_perm,
                    ModifierType.CANNOT_DIGIVOLVE,
                    value_fn=lambda: True,
                    expiry='end_of_opponent_turn',
                )

                # Restriction 2: Delete at end of opponent's turn.
                # Attach an OnEndTurn effect to the played permanent that
                # fires only on the opponent's (not owner's) turn end.
                _played = played_perm
                _owner = player

                def _eot_delete_condition(eot_ctx):
                    # Only fire at end of opponent's turn.
                    if _owner is None or _owner.is_my_turn:
                        return False
                    if _played.top_card is None:
                        return False
                    return _played in _owner.battle_area

                def _eot_delete_process(eot_ctx):
                    if _played in _owner.battle_area:
                        _owner.delete_permanent(_played)

                eot_effect = ICardEffect()
                eot_effect.set_timing(EffectTiming.OnEndTurn)
                eot_effect.set_effect_name(
                    "BT23-037 Delete played Hudie at end of opponent's turn"
                )
                eot_effect.set_effect_description(
                    "Delete the played Digimon at the end of your opponent's turn."
                )
                eot_effect.set_can_use_condition(_eot_delete_condition)
                eot_effect.set_on_process_callback(_eot_delete_process)
                _played.grant_temp_effect(eot_effect)

            game.effect_play_from_zone(
                player,
                'hand',
                play_filter,
                free=True,
                is_optional=True,
                prompt="Select 1 [Hudie] Digimon (cost ≤5) to play without paying cost.",
                on_played=on_played,
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
