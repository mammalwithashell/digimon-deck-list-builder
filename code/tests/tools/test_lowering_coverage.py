"""Tests for code/tools/lowering_coverage.py (Plan 2 Task 8a)."""
from __future__ import annotations

import json
from pathlib import Path

import pytest

from tools import lowering_coverage as lc

MECH_RS = r'''
//! CompiledStep::Delete in a doc comment must not count.
use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};

const SNIPPET: &str = r#"
card: SNIP
effects:
  - when: when_attacking
    process:
      - draw: { of: you, count: 1 }
"#;

fn helper() -> CompiledPredicate {
    CompiledPredicate { level_eq: Some(3), kind: None, ..Default::default() }
}

fn unused_helper() -> CompiledPredicate {
    CompiledPredicate { trait_has: Some("X".into()), ..Default::default() }
}

#[test]
fn variant_test() {
    let _s = CompiledStep::Draw { of: You, count: 1 };
    let _p = helper();
    let _r = DebugRunner::new();
}

#[test]
fn yaml_test() {
    let _c = compile(SNIPPET); // CompiledStep::Delete in a comment
}

#[test]
fn literal_test() {
    assert_eq!(timing, "on_play");
    let _k = Keyword::Blocker;
}

#[test]
fn fixture_test() {
    r.hand(0, &["FIX_CARD"]);
}
'''

PARSE_RS = r'''
fn parse(yaml: &str) -> PredicateSpec { serde_yml::from_str(yaml).unwrap() }

#[test]
fn parses_trait_has() {
    let p = parse("trait_has: Royal Knight");
    assert!(p.trait_has.is_some());
}
'''

BEHAVIORAL_RS = r'''
#[test]
fn per_card_test() {
    let _s = CompiledStep::Delete { target: Self_ };
    let _y = "delete: { target: self }";
    let _r = DebugRunner::new();
}
'''

FIXTURE_YAML = """card: FIX_CARD
name: Fixture
kind: digimon
effects:
  - when: on_play
    process:
      - draw: { of: you, count: 1 }
"""

CARD_1 = """card: C-1
effects:
  - when: when_attacking
    process:
      - draw: { of: you, count: 1 }
  - kind: grant_keyword
    keyword: Blocker
alt_paths:
  - kind: digivolve
"""

CARD_2 = """card: C-2
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    process:
      - delete: { target: self }
  - kind: aura
    modifier: CannotBeReturnedToHand
  - when: on_play
    # keyword: Barrier  (a comment, not a grant)
"""


def _vocab() -> lc.Vocab:
    return lc.Vocab(
        step_keys={"draw", "delete"},
        predicate_keys={"kind", "level_eq", "trait_has"},
        step_variants={"Draw": "draw", "Delete": "delete"},
        step_spec={"Draw": "draw", "Delete": "delete"},
        timing_variants={"OnPlay": "on_play", "WhenAttacking": "when_attacking"},
        timings={"on_play", "when_attacking"},
        predicate_fields={"kind": "kind", "level_eq": "level_eq", "trait_has": "trait_has"},
        keywords={"Blocker", "Barrier"},
    )


@pytest.fixture()
def synthetic(tmp_path: Path) -> dict:
    engine = tmp_path / "engine"
    (engine / "tests" / "dsl" / "fixtures").mkdir(parents=True)
    (engine / "tests" / "dsl" / "mech.rs").write_text(MECH_RS, encoding="utf-8")
    (engine / "tests" / "dsl" / "parse_predicates.rs").write_text(PARSE_RS, encoding="utf-8")
    (engine / "tests" / "dsl" / "fixtures" / "FIX_CARD.yaml").write_text(FIXTURE_YAML, encoding="utf-8")
    (engine / "tests" / "cards_behavioral" / "c").mkdir(parents=True)
    (engine / "tests" / "cards_behavioral" / "c" / "c_2.rs").write_text(BEHAVIORAL_RS, encoding="utf-8")
    cards_dir = tmp_path / "cards"
    cards_dir.mkdir()
    (cards_dir / "c1.yaml").write_text(CARD_1, encoding="utf-8")
    (cards_dir / "c2.yaml").write_text(CARD_2, encoding="utf-8")
    impact = {
        "cards": {
            "C-1": {"path": "c1.yaml", "verbs": ["draw"], "predicates": ["level_eq"], "timings": ["when_attacking"]},
            "C-2": {"path": "c2.yaml", "verbs": ["delete"], "predicates": ["trait_has"], "timings": ["on_play"]},
        }
    }
    deck_library = {
        "archetypes": {
            "A": {
                "decklists": [
                    {"format": "EX13", "decklist": json.dumps(["C-1", "C-2", "C-2"])},
                    {"format": "EX13", "decklist": ["C-1"]},
                    {"format": "BT20", "decklist": json.dumps(["C-2"])},
                ]
            }
        }
    }
    kwargs = dict(
        engine=engine,
        impact=impact,
        cards_dir=cards_dir,
        deck_library=deck_library,
        lowering_map={"step/x.rs": ["draw", "delete"]},
        has_card_test={"C-1"},
    )
    return {"kwargs": kwargs, "payload": lc.build(vocab=_vocab(), **kwargs)}


def test_variant_and_same_file_helper_attribution(synthetic):
    c = synthetic["payload"]["constructs"]
    assert "dsl::mech.rs::variant_test" in c["verb:draw"]["tests"]
    assert c["verb:draw"]["signals"]["dsl::mech.rs::variant_test"] == ["variant"]
    # CompiledPredicate field reached through the same-file helper; `kind: None` does not count.
    assert c["predicate:level_eq"]["tests"] == ["dsl::mech.rs::variant_test"]
    assert "dsl::mech.rs::variant_test" not in c["predicate:kind"]["tests"]


def test_yaml_key_attribution_via_referenced_const(synthetic):
    c = synthetic["payload"]["constructs"]
    assert c["verb:draw"]["signals"]["dsl::mech.rs::yaml_test"] == ["yaml_key"]
    assert c["timing:when_attacking"]["signals"]["dsl::mech.rs::yaml_test"] == ["yaml_key"]


def test_timing_literal_keyword_and_fixture(synthetic):
    c = synthetic["payload"]["constructs"]
    assert c["timing:on_play"]["signals"]["dsl::mech.rs::literal_test"] == ["literal"]
    assert c["keyword:Blocker"]["signals"]["dsl::mech.rs::literal_test"] == ["keyword"]
    assert c["timing:on_play"]["signals"]["dsl::mech.rs::fixture_test"] == ["fixture"]
    assert c["verb:draw"]["signals"]["dsl::mech.rs::fixture_test"] == ["fixture"]


def test_comments_and_cards_behavioral_are_ignored(synthetic):
    c = synthetic["payload"]["constructs"]
    # `delete` appears only in comments and in a cards_behavioral test.
    assert c["verb:delete"]["tests"] == []
    assert c["verb:delete"]["tested"] is False
    assert all("cards_behavioral" not in t for v in c.values() for t in v["tests"])
    engine = synthetic["kwargs"]["engine"]
    path = engine / "tests" / "cards_behavioral" / "c" / "c_2.rs"
    assert lc.scan_file(path, engine, _vocab()) == (False, {})


def test_parse_only_classification(synthetic):
    c = synthetic["payload"]["constructs"]
    th = c["predicate:trait_has"]
    assert th["tests"] == ["dsl::parse_predicates.rs::parses_trait_has"]
    assert th["tested"] and not th["tested_behaviourally"] and th["parsed_only"]
    assert c["verb:draw"]["tested_behaviourally"]
    assert "tests/dsl/parse_predicates.rs" in synthetic["payload"]["summary"]["parse_only_files"]


def test_card_rollup_and_list_blocking(synthetic):
    p = synthetic["payload"]
    c1, c2 = p["cards"]["C-1"], p["cards"]["C-2"]
    assert c1["constructs"] == ["verb:draw", "predicate:level_eq", "timing:when_attacking", "keyword:Blocker"]
    assert c1["lowering_covered"] and c1["behaviourally_covered"] and c1["untested_constructs"] == []
    # Barrier is only in a YAML comment, so it is not a construct of C-2.
    assert c2["untested_constructs"] == ["verb:delete"]
    assert c2["parsed_only_constructs"] == ["predicate:trait_has"]
    assert not c2["lowering_covered"]
    top = {t["construct"]: t for t in p["summary"]["top_untested"]}
    assert top["verb:delete"] == {"construct": "verb:delete", "cards_blocked": 1, "ex13_lists_blocked": 1}
    assert p["summary"]["cards"] == {"total": 2, "lowering_covered": 1, "behaviourally_covered": 1, "not_covered": 1}


def test_attention_fields(synthetic):
    cards = synthetic["payload"]["cards"]
    assert cards["C-1"]["clause_count"] == 3 and cards["C-1"]["effect_clause_count"] == 2
    assert cards["C-1"]["protection_constructs"] == []
    assert cards["C-1"]["has_card_test"] and not cards["C-2"]["has_card_test"]
    assert cards["C-2"]["protection_constructs"] == [
        "clause:replacement",
        "modifier:CannotBeReturnedToHand",
        "trigger:when_would_leave_battle_area",
    ]


def test_build_is_deterministic(synthetic):
    again = lc.build(vocab=_vocab(), **synthetic["kwargs"])
    assert lc.serialise(again) == lc.serialise(synthetic["payload"])
    assert "generated_at" not in lc.serialise(again)


def test_check_mode(tmp_path, monkeypatch, synthetic):
    monkeypatch.setattr(lc, "build", lambda: synthetic["payload"])
    out = tmp_path / "lowering_coverage.json"
    assert lc.main(["--check", "--out", str(out)]) == 1  # missing
    assert lc.main(["--out", str(out)]) == 0
    assert lc.main(["--check", "--out", str(out)]) == 0
    out.write_text(out.read_text(encoding="utf-8").replace("C-1", "C-9"), encoding="utf-8")
    assert lc.main(["--check", "--out", str(out)]) == 1


def test_lexer_masks_comments_and_keeps_strings():
    lexed = lc.lex_rust('let a = "x // y"; // CompiledStep::Draw\n/* CompiledStep::Delete */ let c = \'"\';')
    assert "CompiledStep" not in lexed.masked
    assert [s for (_a, _b, s) in lexed.strings] == ["x // y"]


def test_card_keywords_ignores_comments():
    assert lc.card_keywords(CARD_1) == {"Blocker"}
    assert lc.card_keywords(CARD_2) == set()
    assert lc.card_keywords("dual:\n  option:\n    keywords: [ArtsDigivolve, Save]\n") == {"ArtsDigivolve", "Save"}
