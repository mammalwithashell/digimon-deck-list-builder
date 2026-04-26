from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_165(CardScript):
    """P-165 ShoeShoemon | Lv.4 Yellow Puppet/LIBERATOR DP 4000 Cost 4

    [Security] At the end of the battle, play this card without paying the cost.
    [On Play] [When Digivolving] Play 1 [Familiar] Token.
        At the end of your opponent's turn, delete that token.
    Inherited: <Barrier>

    C# ref: PlaySelfDigimonAfterBattleSecurityEffect,
            PlaySelfDeleteFamiliarToken, BarrierSelfEffect
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Security] At the end of the battle, play this card ---
        # C# uses PlaySelfDigimonAfterBattleSecurityEffect which plays the
        # Digimon after the security battle resolves.  In our engine the
        # security_attack flow is: fire SecuritySkill effects -> battle ->
        # trash unless _security_played.  We play the card during the
        # SecuritySkill callback and set _security_played so it is not
        # trashed afterward.  (The card still battles as a Security Digimon
        # since is_digimon is evaluated on the CardSource before the effect.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.SecuritySkill)
        effect0.set_effect_name("P-165 Security: play this card after battle")
        effect0.set_effect_description(
            "[Security] At the end of the battle, play this card without "
            "paying the cost."
        )
        effect0.is_security_effect = True
        effect0._security_play_self = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            game.effect_play_from_security(player, card)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Shared helper: play Familiar Token + schedule EOT deletion ---
        def _play_familiar_and_schedule_deletion(ctx: Dict[str, Any]):
            """Play 1 Familiar Token and schedule its deletion at end of opponent's turn.

            C# ref: CardEffectCommons.PlaySelfDeleteFamiliarToken
            The token type is SelfDeleteFamiliarToken which has an OnEndTurn
            effect that deletes itself when IsOpponentTurn is true.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Track field before to identify the new token
            before_ids = set(id(p) for p in player.battle_area)
            game.effect_play_token(player, 'familiar')

            # Find newly created token(s)
            new_tokens = [p for p in player.battle_area if id(p) not in before_ids]
            for token_perm in new_tokens:
                # Create an OnEndTurn effect that deletes the token at end
                # of opponent's turn.  Inject it into the token's cached
                # effects so the engine picks it up via effect_list().
                eot_effect = ICardEffect()
                eot_effect.set_timing(EffectTiming.OnEndTurn)
                eot_effect.set_effect_name(
                    "P-165 Delete Familiar Token at end of opponent's turn"
                )

                _perm = token_perm  # capture for closures

                def eot_cond(context: Dict[str, Any], p=_perm) -> bool:
                    owner = card.owner if card else None
                    if not owner:
                        return False
                    # Only fire at end of opponent's turn
                    if owner.is_my_turn:
                        return False
                    return p in owner.battle_area

                def eot_proc(ctx: Dict[str, Any], p=_perm):
                    owner = card.owner if card else None
                    if owner and p in owner.battle_area:
                        owner.delete_permanent(p)

                eot_effect.set_can_use_condition(eot_cond)
                eot_effect.set_on_process_callback(eot_proc)

                # Inject into the token's CardSource cached effects so the
                # engine sees it (same pattern as EX11-022).
                if _perm.top_card:
                    _perm.top_card._cached_effects = (
                        _perm.top_card._cached_effects or []
                    )
                    _perm.top_card._cached_effects.append(eot_effect)

        # --- Effect 1: [When Digivolving] Play 1 Familiar Token ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("P-165 Play 1 [Familiar] Token")
        effect1.set_effect_description(
            "[When Digivolving] Play 1 [Familiar] Token. At the end of your "
            "opponent's turn, delete that token."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_play_familiar_and_schedule_deletion)
        effects.append(effect1)

        # --- Effect 2: [On Play] Play 1 Familiar Token ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("P-165 Play 1 [Familiar] Token")
        effect2.set_effect_description(
            "[On Play] Play 1 [Familiar] Token. At the end of your "
            "opponent's turn, delete that token."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_play_familiar_and_schedule_deletion)
        effects.append(effect2)

        # --- Effect 3: Inherited <Barrier> ---
        effect3 = ICardEffect()
        effect3.set_effect_name("P-165 Barrier")
        effect3.set_effect_description("Barrier")
        effect3.is_inherited_effect = True
        effect3._is_barrier = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
