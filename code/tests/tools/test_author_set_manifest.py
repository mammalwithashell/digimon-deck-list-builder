"""Tests for the DCGO keyword-manifest extractor (tasks 2.1 / 2.1b / 2.4)."""

import os

import pytest

from tools.author_set.dcgo_manifest import (
    CORE_MODELED_ALLOWLIST,
    build_manifest,
    classify_keyword_complexity,
    diff_registries,
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


def test_real_manifest_flags_link_as_auto_ingest():
    base = _base_dcgo_or_skip()
    m = build_manifest(base_dcgo=base, rust_enum_path="code/digimon-engine/src/enums.rs")
    # Link is in DCGO but not (yet) the Rust enum -> standing auto-ingest candidate.
    assert "link" in m["auto_ingest_candidates"]
    # The core-modeled blind-spot keywords must show up on the Rust-only side.
    assert "drawx" in m["rust_only_core_modeled"]


def test_real_manifest_classifies_link_as_subsystem_not_simple():
    base = _base_dcgo_or_skip()
    m = build_manifest(base_dcgo=base, rust_enum_path="code/digimon-engine/src/enums.rs")
    # Link is an ActivateClass subsystem — must NOT be offered as a cheap auto-port.
    assert "link" in m["auto_ingest_subsystem"]
    assert "link" not in m["auto_ingest_simple"]
    # Sanity: the union still equals the full candidate list.
    assert set(m["auto_ingest_simple"]) | set(m["auto_ingest_subsystem"]) == set(m["auto_ingest_candidates"])
