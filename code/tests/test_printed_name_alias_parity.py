"""Printed name aliases must be authored on every implemented card.

A card that prints "(Rule) Name: Also treated as [X]." or "The name of this card/Digimon is
also treated as [X]." HAS the name [X] wherever it is (CR 4.3 2-3-1-2: a brackets-only
reference matches a card with that name). DCGO scripts it as an unconditional
`ChangeCardNamesClass` / `NameIsAlso`.

In this engine a name alias reaches the game only through the card's YAML spec
(`also_treated_as`); `cards.json` cannot carry it because the digimoncard.io API drops
(Rule) lines. So a spec without the field silently has only its printed name, and every
exact-name gate (`name_is`, `name_in`, the brackets-only "[Digivolve] [Name]: Cost N" routes
that the 2026-11 name-gate audit turned from substring into exact matches) skips the card.
Shoutmon (King Version) (BT10-111), "also treated as [Shoutmon]", was such a card: the
[Shoutmon] routes of OmniShoutmon (BT19-012) and OmniShoutmon (BT21-021) had matched it only
because the gate was a substring test.

Scope: aliases that are part of the card's identity. Excluded on purpose, because they are
scoped by the printed text and modelled by other fields:
  * "... is also treated as [X] for DigiXros" -> `digixros_aliases`;
  * "... is also treated as Lv.6 [X] for [Examon]'s DNA digivolution" -> a
    `dna_material_identity` aura;
  * conditional, effect-granted names ("for the turn, ...").
"""
import glob
import os
import re

import yaml

_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.abspath(os.path.join(_HERE, "..", ".."))
_BUNDLES = os.path.join(_ROOT, "data", "card_bundles")
_CARDS = os.path.join(_ROOT, "code", "digimon-engine", "cards")

# "(Rule) Name: Also treated as [A]/[B]."  |  "The name of this card/Digimon is also treated as [A] and [B]."
_ALIAS_SENTENCE = re.compile(
    r"(?:\(Rule\)\s*Name:\s*Also treated as|[Tt]he name of this card/Digimon is also treated as)"
    r"\s*((?:\[[^\]]+\]\s*(?:/|and|or)?\s*)+)\."
)
_BRACKETED = re.compile(r"\[([^\]]+)\]")


def _printed_text(bundle_path):
    """The official printed-text section of a card bundle (no Q&A: rulings quote other cards)."""
    with open(bundle_path, encoding="utf-8") as f:
        text = f.read()
    text = text.split("## Official Q&A")[0]
    return text.split("## Printed text")[-1]


def printed_name_aliases(text):
    """Every identity alias the printed text grants, in printed order."""
    names = []
    for match in _ALIAS_SENTENCE.finditer(text):
        for name in _BRACKETED.findall(match.group(1)):
            if name not in names:
                names.append(name)
    return names


def _spec_paths():
    out = {}
    for path in glob.glob(os.path.join(_CARDS, "*", "*.yaml")):
        set_dir = os.path.basename(os.path.dirname(path))
        if set_dir.startswith("_") or set_dir == "test":
            continue
        out[os.path.splitext(os.path.basename(path))[0]] = path
    return out


def _authored_aliases(spec_path):
    with open(spec_path, encoding="utf-8") as f:
        spec = yaml.safe_load(f)
    aliases = (spec or {}).get("also_treated_as") or []
    return [aliases] if isinstance(aliases, str) else list(aliases)


def test_the_alias_sentence_parser_reads_both_printed_forms():
    assert printed_name_aliases(
        "... may place. (Rule) Name: Also treated as [Takuya Kanbara]/[Koji Minamoto]."
    ) == ["Takuya Kanbara", "Koji Minamoto"]
    assert printed_name_aliases(
        "The name of this card/Digimon is also treated as [MetalGreymon] and [Cyberdramon]. <Armor Purge>"
    ) == ["MetalGreymon", "Cyberdramon"]
    # Scoped aliases are other fields, not identity aliases.
    assert printed_name_aliases("This card is also treated as [Shoutmon] for DigiXros.") == []
    assert (
        printed_name_aliases(
            "[All Turns] This Digimon is also treated as Lv.6 [Slayerdramon] for [Examon]'s DNA digivolution."
        )
        == []
    )


def test_every_implemented_card_authors_its_printed_name_aliases():
    specs = _spec_paths()
    checked = 0
    missing = []
    for bundle in sorted(glob.glob(os.path.join(_BUNDLES, "*.md"))):
        card_id = os.path.splitext(os.path.basename(bundle))[0]
        if card_id not in specs:
            continue
        printed = printed_name_aliases(_printed_text(bundle))
        if not printed:
            continue
        checked += 1
        authored = _authored_aliases(specs[card_id])
        lacking = [name for name in printed if name not in authored]
        if lacking:
            missing.append(f"{card_id}: prints also-treated-as {printed}, spec lacks {lacking}")
    # The walk must actually see the known alias cards (a moved bundle dir or a changed
    # bundle layout would otherwise pass vacuously).
    assert checked >= 10, f"only {checked} implemented cards with a printed name alias were found"
    assert not missing, (
        "implemented cards whose YAML lacks a printed name alias (`also_treated_as`):\n  "
        + "\n  ".join(missing)
    )
