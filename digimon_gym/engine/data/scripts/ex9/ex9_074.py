from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_074(CardScript):
    """EX9-074 Kimeramon | Lv.5 | Composite/DM/Ver.3
    <Rush>
    <Security A. +1>
    [On Play][When Digivolving] You may place 1 Lv.4 or lower [DM] Digimon from trash
    as this Digimon's top digivolution card. Then, delete 1 opponent Digimon with same
    color as any digivolution card. If 6+ colors in digi cards, instead delete 1 of each
    opponent's Digimon with different colors.
    [All Turns] +1000 DP per color in digi cards.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Rush>
        effect_rush = ICardEffect()
        effect_rush.set_effect_name("EX9-074 Rush")
        effect_rush.set_effect_description("Rush")
        effect_rush._is_rush = True

        def cond_rush(context: Dict[str, Any]) -> bool:
            return True
        effect_rush.set_can_use_condition(cond_rush)
        effects.append(effect_rush)

        # <Security A. +1>
        effect_sa = ICardEffect()
        effect_sa.set_effect_name("EX9-074 Security A. +1")
        effect_sa.set_effect_description("Security A. +1")
        effect_sa._security_attack_modifier = 1

        def cond_sa(context: Dict[str, Any]) -> bool:
            return True
        effect_sa.set_can_use_condition(cond_sa)
        effects.append(effect_sa)

        def _get_digi_colors(perm):
            """Get all colors from digivolution cards (NOT top card).
            C#: DigivolutionCards.Filter(x => !x.IsFlipped).SelectMany(e => e.CardColors).Distinct()"""
            colors = set()
            for src in perm.card_sources[:-1]:  # Exclude top card
                card_colors = getattr(src, 'card_colors', []) or []
                for col in card_colors:
                    colors.add(col)
            return colors

        def _dm_trash_filter(c):
            """Lv.4 or lower [DM] Digimon from trash."""
            if not getattr(c, 'is_digimon', False):
                return False
            level = getattr(c, 'level', None)
            if level is None or level > 4:
                return False
            traits = getattr(c, 'card_traits', []) or []
            return any('DM' in t for t in traits)

        def _place_and_delete(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Step 1: Optionally place 1 Lv.4 or lower DM Digimon from trash as top digi card
            # C#: canNoSelect: () => true — optional selection via SelectTrash
            from ....game.constants import SEL_TRASH_START
            valid_trash = []
            for i, c in enumerate(player.trash_cards):
                if _dm_trash_filter(c):
                    valid_trash.append(SEL_TRASH_START + i)

            def _after_place():
                """After optional placement, proceed to deletion step."""
                colors = _get_digi_colors(perm)

                if len(colors) >= 6:
                    # C#: iterate each color, select 1 opponent Digimon per color (each unique)
                    deleted_perms = []

                    def _delete_by_colors(color_list, idx=0):
                        if idx >= len(color_list):
                            # Batch delete collected permanents
                            for dp in deleted_perms:
                                if dp in enemy.battle_area:
                                    enemy.delete_permanent(dp)
                            return
                        col = color_list[idx]

                        def col_filter(p):
                            if not p.is_digimon:
                                return False
                            if p in deleted_perms:
                                return False
                            opp_colors = getattr(p.top_card, 'card_colors', []) or [] if p.top_card else []
                            return col in opp_colors

                        has_target = any(col_filter(p) for p in enemy.battle_area)
                        if not has_target:
                            _delete_by_colors(color_list, idx + 1)
                            return

                        def on_select(target_perm):
                            if target_perm:
                                deleted_perms.append(target_perm)
                            _delete_by_colors(color_list, idx + 1)

                        game.effect_select_opponent_permanent(
                            player, on_select, filter_fn=col_filter, is_optional=False)

                    # Determine which colors have deletable opponent Digimon
                    color_names = set()
                    for opp in enemy.battle_area:
                        if opp.is_digimon and opp.top_card:
                            for col in (getattr(opp.top_card, 'card_colors', []) or []):
                                color_names.add(col)
                    deletable_colors = sorted(color_names, key=lambda c: str(c))
                    _delete_by_colors(deletable_colors)
                else:
                    # Delete 1 opponent Digimon with same color as any digi card
                    def color_filter(p):
                        if not p.is_digimon:
                            return False
                        opp_colors = getattr(p.top_card, 'card_colors', []) or [] if p.top_card else []
                        return any(col in colors for col in opp_colors)

                    has_target = any(color_filter(p) for p in enemy.battle_area)
                    if has_target:
                        def on_delete(target_perm):
                            enemy.delete_permanent(target_perm)

                        game.effect_select_opponent_permanent(
                            player, on_delete, filter_fn=color_filter, is_optional=False)

            if valid_trash:
                def on_trash_selected(action_id):
                    idx = action_id - SEL_TRASH_START
                    if idx < len(player.trash_cards):
                        selected = player.trash_cards[idx]
                        player.trash_cards.remove(selected)
                        # C#: AddDigivolutionCardsTop — place as top digi card (just under top)
                        perm.card_sources.insert(len(perm.card_sources) - 1, selected)
                    _after_place()

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_selected,
                    valid_trash, is_optional=True,
                    prompt="Select 1 Lv.4 or lower [DM] Digimon from trash to place as top digivolution card.")
            else:
                _after_place()

        # [On Play]
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX9-074 On Play: Place DM from trash, color delete")
        effect2.set_effect_description("[On Play] Place DM from trash, delete by color.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_place_and_delete)
        effects.append(effect2)

        # [When Digivolving]
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX9-074 When Digivolving: Place DM from trash, color delete")
        effect3.set_effect_description("[When Digivolving] Place DM from trash, delete by color.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_place_and_delete)
        effects.append(effect3)

        # [All Turns] +1000 DP per color in digi cards
        # C#: ChangeSelfDPStaticEffect with dynamic value = 1000 * DigivolutionCardsColors.Count
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.NoTiming)
        effect4.set_effect_name("EX9-074 All Turns: +1000 DP per color")
        effect4.set_effect_description("[All Turns] +1000 DP per color in digi cards.")

        def condition4(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            return len(_get_digi_colors(perm)) >= 1
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Register dynamic DP modifier based on digi card colors."""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.CHANGE_DP,
                    value_fn=lambda: 1000 * len(_get_digi_colors(perm)),
                    expiry='permanent')

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
