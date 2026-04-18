from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from ....interfaces.modifiers import ModifierType

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_020(CardScript):
    """EX6-020 Gatomon | Lv.4 Yellow Holy Beast

    [On Play] [When Digivolving] Reveal the top 3 cards of your deck. Add 1 card
    with the [Angel], [Archangel] or [Fallen Angel] trait and 1 [Mirei Mikagura]
    among them to the hand. Return the rest to the bottom of the deck.

    Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
    gets -2000 DP for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Shared filter helpers for reveal passes ───────────────────────
        def is_angel_family_digimon(c) -> bool:
            if not getattr(c, 'is_digimon', False):
                return False
            traits = list(getattr(c, 'card_traits', []) or [])
            return (
                'Angel' in traits
                or 'Archangel' in traits
                or 'Fallen Angel' in traits
                or 'FallenAngel' in traits
            )

        def is_mirei_mikagura_tamer(c) -> bool:
            if not getattr(c, 'is_tamer', False):
                return False
            names = getattr(c, 'card_names', []) or []
            return any(
                n == 'Mirei Mikagura' or n == 'MireiMikagura' for n in names
            )

        def make_reveal_condition():
            def _cond(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                if not owner:
                    return False
                return len(owner.library_cards) >= 1
            return _cond

        def make_reveal_process():
            def _process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return
                game.effect_reveal_and_select_multi(
                    player,
                    count=3,
                    passes=[
                        (is_angel_family_digimon, 'hand'),
                        (is_mirei_mikagura_tamer, 'hand'),
                    ],
                    remaining_placement='deck_bottom',
                    is_optional=False,
                )
            return _process

        # ── Effect 0: [On Play] reveal 3 + 2 selection passes ─────────────
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name(
            "EX6-020 Reveal top 3: add Angel/Archangel/Fallen Angel + Mirei Mikagura"
        )
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with the "
            "[Angel], [Archangel] or [Fallen Angel] trait and 1 [Mirei Mikagura] "
            "among them to the hand. Return the rest to the bottom of the deck."
        )
        effect0.is_on_play = True
        effect0.set_can_use_condition(make_reveal_condition())
        effect0.set_on_process_callback(make_reveal_process())
        effects.append(effect0)

        # ── Effect 1: [When Digivolving] same effect (separate instance) ──
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name(
            "EX6-020 Reveal top 3: add Angel/Archangel/Fallen Angel + Mirei Mikagura"
        )
        effect1.set_effect_description(
            "[When Digivolving] Reveal the top 3 cards of your deck. Add 1 card "
            "with the [Angel], [Archangel] or [Fallen Angel] trait and 1 "
            "[Mirei Mikagura] among them to the hand. Return the rest to the "
            "bottom of the deck."
        )
        effect1.is_when_digivolving = True
        effect1.set_can_use_condition(make_reveal_condition())
        effect1.set_on_process_callback(make_reveal_process())
        effects.append(effect1)

        # ── Effect 2: Inherited [When Attacking][OPT] -2000 DP ────────────
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX6-020 DP -2000")
        effect2.set_effect_description(
            "[When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets "
            "-2000 DP for the turn."
        )
        effect2.is_inherited_effect = True
        effect2.is_on_attack = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("DP-2000_EX6-020")

        def condition2(context: Dict[str, Any]) -> bool:
            # Must be on field
            if not card:
                return False
            host = card.permanent_of_this_card()
            if host is None:
                return False
            # Must be THIS permanent attacking (filter out other allies' attacks)
            attacker = context.get('attacker')
            if attacker is not None and attacker is not host:
                return False
            # Must be own turn's attack
            event_player = context.get('event_player')
            owner = card.owner
            if event_player is not None and owner is not None and event_player is not owner:
                return False
            # Opponent must have at least one Digimon to target
            enemy = owner.enemy if owner else None
            if not enemy:
                return False
            has_target = any(
                p.is_digimon and p.dp is not None for p in enemy.battle_area
            )
            return has_target

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_target(target_perm):
                if target_perm is None:
                    return
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_DP,
                    value_fn=lambda current, t, c: current - 2000,
                    condition=lambda t, c, _tp=target_perm: t is _tp,
                    expiry='end_of_turn',
                )

            game.effect_select_opponent_permanent(
                player, on_target,
                filter_fn=lambda p: p.is_digimon and p.dp is not None,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to get -2000 DP for the turn.",
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
