"""Tests for the DCGO keyword-manifest extractor (tasks 2.1 / 2.1b / 2.4)."""

import os

import pytest

from tools.author_set.dcgo_manifest import (
    CORE_MODELED_ALLOWLIST,
    DSL_KEYWORD_VOCAB,
    build_manifest,
    classify_keyword_complexity,
    diff_registries,
    dsl_lowered_keywords,
    extract_dcgo_keywords,
    find_base_dcgo,
    interface_keywords_from_text,
    keywords_from_filenames,
    normalize_keyword,
    parse_rust_enum_keywords,
)


def test_complexity_classifier_distinguishes_subsystem_from_flag():
    # Link-shaped: activated effect with player selection touching board state.
    link_src = "new ActivateClass(); ... SelectPermanentEffect ... new ILinkCard(...).LinkCard()"
    assert classify_keyword_complexity(link_src) == "subsystem"
    # Rush/Blocker-shaped: passive static flag.
    flag_src = "StaticEffect ... ICardEffect ... StaticEffect"
    assert classify_keyword_complexity(flag_src) == "simple"
    assert classify_keyword_complexity("") == "simple"

RUST_ENUM_SNIPPET = """
pub enum Keyword {
    Blocker,
    SecurityAttackPlus(i8),
    Rush,
    Piercing,
    DrawX(u8),
    BlastDigivolve,
    // a comment line
    Vortex,
    MindLink,
    ArtsDigivolve,
}
"""


def test_normalize_aliases_dcgo_to_rust():
    assert normalize_keyword("Pierce") == "piercing"
    assert normalize_keyword("BlastDigivolution") == "blastdigivolve"
    assert normalize_keyword("BlastDNADigivolution") == "blastdnadigivolve"
    assert normalize_keyword("Rush") == "rush"


def test_neither_keyword_dir_is_complete_union_recovers_both():
    # MindLink only in Commons; Link only in Factory (the audited reality).
    factory = ["Rush.cs", "Link.cs", "ArtsDigivolve.cs", "Blocker.cs"]
    commons = ["Rush.cs", "MindLink.cs", "Blocker.cs"]
    union = keywords_from_filenames(factory) | keywords_from_filenames(commons)
    assert "link" in union
    assert "mindlink" in union
    assert "rush" in union


def test_meta_files_ignored():
    kws = keywords_from_filenames(["Rush.cs", "Rush.cs.meta"])
    assert kws == {"rush"}


def test_interface_keyword_extraction():
    text = (
        "public interface IRushEffect {}\n"
        "public interface IIcecladEffect {}\n"
        "public interface IBlockerEffect {}\n"
        "public interface ISomethingElse {}\n"  # not I*Effect-keyword shaped is still captured
    )
    kws = interface_keywords_from_text(text)
    assert {"rush", "iceclad", "blocker"}.issubset(kws)


def test_parse_rust_enum_keywords():
    kws = parse_rust_enum_keywords(RUST_ENUM_SNIPPET)
    assert "blocker" in kws
    assert "securityattackplus" in kws  # param-carrying variant
    assert "drawx" in kws
    assert "blastdigivolve" in kws
    assert "mindlink" in kws


def test_diff_surfaces_known_auto_ingest_candidates():
    # DCGO registry has link/ascension/blastdnadigivolve that the rust set lacks.
    dcgo = {"rush", "blocker", "link", "ascension", "blastdnadigivolve"}
    rust = {"rush", "blocker"}
    d = diff_registries(dcgo, rust)
    assert d["auto_ingest_candidates"] == ["ascension", "blastdnadigivolve", "link"]


def test_core_modeled_allowlist_is_the_directory_blind_spot():
    # These Rust keywords are deliberately NOT in DCGO's KeyWordEffects dirs.
    assert set(CORE_MODELED_ALLOWLIST) == {
        "securityattackplus",
        "securityattackminus",
        "drawx",
        "dedigivolve",
        "digiburst",
    }


# ---- task 9.1: keywords the engine lowers through the DSL ---------------------


def _spec(cards_dir, rel, body):
    p = cards_dir / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(body, encoding="utf-8", newline="\n")


def test_dsl_lowered_keywords_need_a_card_spec_using_the_vocabulary(tmp_path):
    cards = tmp_path / "cards"
    _spec(cards, "p/P-236.yaml", "id: P-236\nuse_requirement:\n  all_turns: true\n"
                                 "clauses:\n  - kind: delay\n    trigger: main\n")
    # Mentioned only in a comment: no evidence the DSL lowers it.
    _spec(cards, "bt26/BT26-032.yaml", "# - kind: succession\nid: BT26-032\n")
    found = dsl_lowered_keywords(cards)
    assert set(found) == {"delay", "usereq"}
    assert all(found[k] == DSL_KEYWORD_VOCAB[k][0] for k in found)


def test_every_dsl_vocab_entry_is_a_normalized_keyword():
    for kw in DSL_KEYWORD_VOCAB:
        assert normalize_keyword(kw) == kw


def _fixture_dcgo(tmp_path):
    root = tmp_path / "DCGO"
    kw_dir = root / "Assets" / "Scripts" / "Script" / "CardEffectFactory" / "KeyWordEffects"
    kw_dir.mkdir(parents=True)
    (kw_dir / "Rush.cs").write_text("StaticEffect", encoding="utf-8")
    (kw_dir / "Link.cs").write_text("new ActivateClass(); SelectPermanentEffect", encoding="utf-8")
    (kw_dir / "Detach.cs").write_text("StaticEffect", encoding="utf-8")
    (kw_dir / "Petrify.cs").write_text("StaticEffect", encoding="utf-8")
    enum = tmp_path / "enums.rs"
    enum.write_text("pub enum Keyword {\n    Rush,\n}\n", encoding="utf-8")
    cards = tmp_path / "cards"
    _spec(cards, "st22/ST22-08.yaml", "clauses:\n  - kind: link_requirement\n    cost: 2\n")
    _spec(cards, "bt26/BT26-010.yaml", "cost:\n  link_card_filter: { trait_has: Seven Code }\n")
    return root, enum, cards


def test_manifest_drops_dsl_lowered_keywords_from_the_port_candidates(tmp_path):
    # Link stays a DCGO subsystem (a fact about DCGO's C#), but the engine
    # lowers it, so it is no longer a port to schedule; Detach likewise is no
    # longer a cheap auto-ingest. Petrify (DCGO only) still is.
    root, enum, cards = _fixture_dcgo(tmp_path)
    m = build_manifest(base_dcgo=str(root), rust_enum_path=str(enum), cards_dir=str(cards))
    assert set(m["dsl_lowered_keywords"]) == {"link", "detach"}
    assert "link" in m["subsystem_keywords"]
    assert m["auto_ingest_candidates"] == ["petrify"]
    assert m["auto_ingest_simple"] == ["petrify"]
    assert m["auto_ingest_subsystem"] == []


def test_committed_manifest_dsl_section_matches_the_committed_card_specs():
    # Guard against rot: every DSL-lowered keyword the committed manifest
    # claims still has a card spec using its vocabulary, and none is missing.
    import json

    with open("data/dcgo_keyword_manifest.json", encoding="utf-8") as f:
        committed = json.load(f)
    assert committed["dsl_lowered_keywords"] == dsl_lowered_keywords("code/digimon-engine/cards")


# ---- integration against the real base-repo DCGO (skipped if absent) --------

def _base_dcgo_or_skip():
    try:
        base = find_base_dcgo()
    except Exception:  # pragma: no cover
        pytest.skip("git not available")
    if not os.path.isdir(os.path.join(base, "Assets")):
        pytest.skip("base-repo DCGO not populated")
    return base


def test_real_dcgo_registry_is_complete():
    base = _base_dcgo_or_skip()
    registry, interfaces = extract_dcgo_keywords(base)
    # Audited union = 33 keywords; allow growth but not regression.
    assert len(registry) >= 33
    # Spot-check the keywords that live in only one of the two dirs.
    assert "link" in registry
    assert "mindlink" in registry
    assert "rush" in registry


def test_real_manifest_no_longer_offers_dsl_lowered_link_as_a_port():
    base = _base_dcgo_or_skip()
    m = build_manifest(base_dcgo=base, rust_enum_path="code/digimon-engine/src/enums.rs")
    # Link is in DCGO and not the Rust enum, but the DSL lowers it (DigiLink
    # substrate) -> covered, not an auto-ingest candidate any more (task 9.1).
    assert "link" in m["dsl_lowered_keywords"]
    assert "link" not in m["auto_ingest_candidates"]
    # The core-modeled blind-spot keywords must show up on the Rust-only side.
    assert "drawx" in m["rust_only_core_modeled"]


def test_real_manifest_still_classifies_dcgo_link_as_subsystem():
    base = _base_dcgo_or_skip()
    m = build_manifest(base_dcgo=base, rust_enum_path="code/digimon-engine/src/enums.rs")
    # DCGO's Link is an ActivateClass subsystem; that classification is kept
    # (it is a fact about DCGO's C#) even though the engine no longer needs it.
    assert "link" in m["subsystem_keywords"]
    assert "link" not in m["auto_ingest_subsystem"] and "link" not in m["auto_ingest_simple"]
    # Sanity: the union still equals the full candidate list.
    assert set(m["auto_ingest_simple"]) | set(m["auto_ingest_subsystem"]) == set(m["auto_ingest_candidates"])
