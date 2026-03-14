from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_021(CardScript):
    """EX9-021 Omnimon Alter-S | Lv.7 Blue/Red Digimon | Holy Knight/DM

    Alt digivolve: Lv.6 w/ [DM] trait for cost 5.

    DNA Digivolution: Blue Lv.6 + Red Lv.6 : Cost 0

    [When Digivolving] If DNA digivolving, your opponent's effects don't
        affect this Digimon for the turn. Then, delete all of their Digimon
        with the highest level.

    [End of Attack] You may play 1 card with [Greymon] in its name or the
        [Ver.1] trait and 1 card with [Garurumon] in its name or the [Ver.2]
        trait from this Digimon's digivolution cards without paying the costs.
        If this effect played, place this Digimon as your top security card.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        from ....interfaces.modifiers import ModifierType

        effects = []

        def _has_trait(c, trait_name: str) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any(trait_name in t for t in traits)

        # --- Effect 0: Alt digivolve from Lv.6 with [DM] trait for cost 5 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-021 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution: Lv.6 [DM] trait for cost 5")
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 6
        effect0._alt_digi_trait = "DM"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: DNA Digivolution: Blue Lv.6 + Red Lv.6 : Cost 0 ---
        # DNA digivolution conditions are encoded on the card entity's dna_costs
        # in the card database, not in the script. No script-side effect needed.

        # --- Effect 2: [When Digivolving] If DNA, grant immunity + delete
        #     all opponent Digimon with the highest level ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX9-021 When Digivolving: DNA immunity + delete highest level")
        effect2.set_effect_description(
            "[When Digivolving] If DNA digivolving, your opponent's effects "
            "don't affect this Digimon for the turn. Then, delete all of their "
            "Digimon with the highest level."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if not perm.is_digimon:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy
            if not enemy:
                return

            # If DNA digivolving, grant immunity to opponent effects for the turn
            is_dna = ctx.get('is_dna_digivolve', False)
            if is_dna:
                game.register_modifier(
                    perm,
                    ModifierType.CANNOT_BE_AFFECTED,
                    value_fn=lambda: True,
                    expiry='end_of_turn',
                )

            # Delete all opponent Digimon with the highest level
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return

            def _get_level(p):
                if p.top_card:
                    return getattr(p.top_card, 'level', 0) or 0
                return 0

            max_level = max(_get_level(p) for p in opp_digimon)
            to_delete = [p for p in opp_digimon if _get_level(p) == max_level]
            for p in to_delete:
                if p in enemy.battle_area:
                    enemy.delete_permanent(p)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [End of Attack] Play Greymon/Ver.1 + Garurumon/Ver.2
        #     from evo cards free, then place this Digimon as top security ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndAttack)
        effect3.set_effect_name("EX9-021 End of Attack: Play from evo cards, place on security")
        effect3.set_effect_description(
            "[End of Attack] You may play 1 card with [Greymon] in its name or "
            "the [Ver.1] trait and 1 card with [Garurumon] in its name or the "
            "[Ver.2] trait from this Digimon's digivolution cards without "
            "paying the costs. If this effect played, place this Digimon as "
            "your top security card."
        )
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            ctx_perm = context.get('attacking_permanent') or context.get('permanent')
            if perm and ctx_perm and perm is not ctx_perm:
                return False
            if not perm.is_digimon:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            from ....data.enums import GamePhase
            from ....game.constants import SOURCES_PER_FIELD

            def _is_greymon_or_ver1(c) -> bool:
                if not getattr(c, 'is_digimon', False):
                    return False
                if c.contains_card_name('Greymon'):
                    return True
                if _has_trait(c, 'Ver.1'):
                    return True
                return False

            def _is_garurumon_or_ver2(c) -> bool:
                if not getattr(c, 'is_digimon', False):
                    return False
                if c.contains_card_name('Garurumon'):
                    return True
                if _has_trait(c, 'Ver.2'):
                    return True
                return False

            # Find field index of this permanent
            field_idx = None
            for fi, fp in enumerate(player.battle_area):
                if fp is perm:
                    field_idx = fi
                    break
            if field_idx is None:
                return
            base = 2000 + field_idx * SOURCES_PER_FIELD

            played_any = [False]

            def _select_greymon():
                """Let agent select 1 Greymon/Ver.1 from evo cards to play."""
                evo_cards = perm.card_sources[:-1] if len(perm.card_sources) > 1 else []
                greymon_candidates = [c for c in evo_cards if _is_greymon_or_ver1(c)]
                if not greymon_candidates:
                    _select_garurumon()
                    return

                valid = []
                for i, cs in enumerate(perm.card_sources):
                    if cs in greymon_candidates and (base + i) < 2168:
                        valid.append(base + i)
                if not valid:
                    _select_garurumon()
                    return

                def on_select(action_id: int):
                    idx = action_id - base
                    if 0 <= idx < len(perm.card_sources):
                        chosen = perm.card_sources[idx]
                        if chosen in perm.card_sources:
                            perm.card_sources.remove(chosen)
                            played_perm = player.play_card_from_source(
                                chosen, pay_cost=False)
                            if played_perm:
                                played_any[0] = True
                    _select_garurumon()

                game.request_selection(
                    GamePhase.SelectSource, player, on_select, valid,
                    is_optional=False,
                    prompt="Select 1 card with [Greymon] or [Ver.1] trait to play."
                )

            def _select_garurumon():
                """Let agent select 1 Garurumon/Ver.2 from evo cards to play."""
                evo_cards = perm.card_sources[:-1] if len(perm.card_sources) > 1 else []
                garurumon_candidates = [c for c in evo_cards
                                        if _is_garurumon_or_ver2(c)]
                if not garurumon_candidates:
                    _finalize()
                    return

                valid = []
                for i, cs in enumerate(perm.card_sources):
                    if cs in garurumon_candidates and (base + i) < 2168:
                        valid.append(base + i)
                if not valid:
                    _finalize()
                    return

                def on_select(action_id: int):
                    idx = action_id - base
                    if 0 <= idx < len(perm.card_sources):
                        chosen = perm.card_sources[idx]
                        if chosen in perm.card_sources:
                            perm.card_sources.remove(chosen)
                            played_perm = player.play_card_from_source(
                                chosen, pay_cost=False)
                            if played_perm:
                                played_any[0] = True
                    _finalize()

                game.request_selection(
                    GamePhase.SelectSource, player, on_select, valid,
                    is_optional=False,
                    prompt="Select 1 card with [Garurumon] or [Ver.2] trait to play."
                )

            def _finalize():
                """If at least 1 card was played, place this Digimon as
                top security card."""
                if played_any[0] and perm in player.battle_area:
                    player.put_permanent_to_security(perm)

            _select_greymon()

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
