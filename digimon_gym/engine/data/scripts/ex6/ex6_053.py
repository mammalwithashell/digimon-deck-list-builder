from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


def _has_mirei_on_field(player) -> bool:
    """True if player has a [Mirei Mikagura] permanent in battle area."""
    if player is None:
        return False
    for perm in player.battle_area:
        top = perm.top_card
        if top is None:
            continue
        names = getattr(top, 'card_names', []) or []
        if any('Mirei Mikagura' in (n or '') or 'MireiMikagura' in (n or '')
               for n in names):
            return True
    return False


def _is_mirei_source(cs) -> bool:
    """True if a CardSource is a [Mirei Mikagura] card."""
    if cs is None:
        return False
    names = getattr(cs, 'card_names', []) or []
    return any('Mirei Mikagura' in (n or '') or 'MireiMikagura' in (n or '')
               for n in names)


class EX6_053(CardScript):
    """EX6-053 LadyDevimon | Lv.5

    Main effect:
      <Retaliation>
      [On Play] [When Digivolving] If you have a [Mirei Mikagura], delete 1 of
        your opponent's level 4 or lower Digimon. If you don't have a
        [Mirei Mikagura], you may play 1 [Mirei Mikagura] from your trash
        without paying the cost.

    Inherited effect:
      [All Turns] While this Digimon has the [Angel] or [Seven Great Demon
        Lords] trait, it gains <Scapegoat>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── <Retaliation> — main card keyword ───────────────────────
        effect_retaliation = ICardEffect()
        effect_retaliation.set_effect_name("EX6-053 Retaliation")
        effect_retaliation.set_effect_description("Retaliation")
        effect_retaliation.is_on_deletion = True
        effect_retaliation._is_retaliation = True

        def cond_retaliation(context: Dict[str, Any]) -> bool:
            return True
        effect_retaliation.set_can_use_condition(cond_retaliation)
        effects.append(effect_retaliation)

        # ── [On Play] / [When Digivolving] conditional branches ────
        def make_triggered_effect(when_digivolving: bool) -> ICardEffect:
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone)
            bracket = "When Digivolving" if when_digivolving else "On Play"
            eff.set_effect_name(
                f"EX6-053 [{bracket}] Delete Lv.4 or play Mirei Mikagura"
            )
            eff.set_effect_description(
                f"[{bracket}] If you have a [Mirei Mikagura], delete 1 of your "
                "opponent's level 4 or lower Digimon. If you don't have a "
                "[Mirei Mikagura], you may play 1 [Mirei Mikagura] from your "
                "trash without paying the cost."
            )
            if when_digivolving:
                eff.is_when_digivolving = True
            else:
                eff.is_on_play = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True
            eff.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                # Branch A: no Mirei on field → may play 1 Mirei from trash
                if not _has_mirei_on_field(player):
                    # Optional. Only presented if a Mirei exists in trash.
                    def play_filter(cs):
                        return _is_mirei_source(cs)

                    # effect_play_from_zone handles selection from trash,
                    # pays no cost, fires OnEnterFieldAnyone afterwards.
                    game.effect_play_from_zone(
                        player,
                        'trash',
                        play_filter,
                        free=True,
                        is_optional=True,
                        prompt="Select 1 [Mirei Mikagura] to play from trash.",
                    )
                    return

                # Branch B: has Mirei on field → mandatory delete 1
                # opponent Lv.4 or lower Digimon (if a valid target exists).
                def target_filter(perm):
                    if not getattr(perm, 'is_digimon', False):
                        return False
                    lvl = getattr(perm, 'level', None)
                    if lvl is None or lvl > 4:
                        return False
                    return True

                def on_delete(target_perm):
                    enemy = player.enemy
                    if enemy is not None:
                        enemy.delete_permanent(
                            target_perm, is_opponent_effect=True,
                        )

                game.effect_select_opponent_permanent(
                    player,
                    on_delete,
                    filter_fn=target_filter,
                    is_optional=False,
                    prompt="Select 1 opponent Lv.4 or lower Digimon to delete.",
                )

            eff.set_on_process_callback(process)
            return eff

        effects.append(make_triggered_effect(when_digivolving=False))
        effects.append(make_triggered_effect(when_digivolving=True))

        # ── Inherited <Scapegoat> trait-gated ──────────────────────
        effect_scapegoat = ICardEffect()
        effect_scapegoat.set_effect_name("EX6-053 Scapegoat")
        effect_scapegoat.set_effect_description(
            "[All Turns] While this Digimon has the [Angel] or "
            "[Seven Great Demon Lords] trait, it gains <Scapegoat>."
        )
        effect_scapegoat.is_inherited_effect = True
        effect_scapegoat._is_scapegoat = True

        def cond_scapegoat(context: Dict[str, Any]) -> bool:
            # Gate Scapegoat on top-card trait — mirrors C# condition:
            #   card.PermanentOfThisCard().TopCard.CardTraits.Contains("Angel") ||
            #   .Contains("Seven Great Demon Lords") || .Contains("SevenGreatDemonLords")
            perm = context.get('permanent')
            if perm is None and card is not None:
                perm = card.permanent_of_this_card()
            if perm is None or perm.top_card is None:
                return False
            traits = getattr(perm.top_card, 'card_traits', []) or []
            for t in traits:
                if not t:
                    continue
                if 'Angel' in t:
                    return True
                if 'Seven Great Demon Lords' in t:
                    return True
                if 'SevenGreatDemonLords' in t:
                    return True
            return False
        effect_scapegoat.set_can_use_condition(cond_scapegoat)
        effects.append(effect_scapegoat)

        return effects
