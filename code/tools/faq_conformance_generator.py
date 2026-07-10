"""Generate curated rules-FAQ conformance tests.

Only manually approved, scenario-izable entries in
``qa/rules/faq_conformance_cases.json`` become Rust tests. The generated module
is checked in so tier-1 runs do not need Python.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


CASES_JSON = Path("qa/rules/faq_conformance_cases.json")
OUTPUT_RS = Path("code/digimon-engine/tests/rules_faq/generated_conformance.rs")
SCHEMA_VERSION = 1


def load_cases(path: Path = CASES_JSON) -> list[dict[str, Any]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("schema_version") != SCHEMA_VERSION:
        raise ValueError(f"unsupported FAQ conformance schema: {data.get('schema_version')}")
    cases = data.get("cases")
    if not isinstance(cases, list):
        raise ValueError("faq conformance cases must contain a cases list")
    _validate_cases(cases)
    return cases


def _validate_cases(cases: list[dict[str, Any]]) -> None:
    seen: set[str] = set()
    for case in cases:
        case_id = case.get("id")
        if not re.fullmatch(r"[a-z0-9_]+", str(case_id or "")):
            raise ValueError(f"invalid case id: {case_id!r}")
        if case_id in seen:
            raise ValueError(f"duplicate case id: {case_id}")
        seen.add(case_id)
        if not case.get("source"):
            raise ValueError(f"{case_id}: missing source")
        if not case.get("ruling_text"):
            raise ValueError(f"{case_id}: missing ruling_text")
        if case.get("approved") and not case.get("scenario_kind"):
            raise ValueError(f"{case_id}: approved case needs scenario_kind")
        if case.get("scenario_kind") == "text_scope_contains":
            card = case.get("card") or {}
            for field in ("card_id", "name", "kind", "play_cost", "colors", "traits"):
                if field not in card:
                    raise ValueError(f"{case_id}: card missing {field}")
            if not case.get("positive_needles"):
                raise ValueError(f"{case_id}: text scope case needs positive_needles")
        elif case.get("approved"):
            raise ValueError(f"{case_id}: unsupported scenario kind {case.get('scenario_kind')}")


def _rs_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def _rs_string_list(values: list[str]) -> str:
    return "&[" + ", ".join(_rs_string(value) for value in values) + "]"


def _rs_color_list(values: list[str]) -> str:
    return "&[" + ", ".join(f"CardColor::{value}" for value in values) + "]"


def _rs_option(value: Any, rust_type: str) -> str:
    if value is None:
        return "None"
    return f"Some({value}{rust_type})"


def _case_struct(case: dict[str, Any]) -> str:
    card = case["card"]
    return f"""    let case = TextScopeCase {{
        id: {_rs_string(case["id"])},
        source: {_rs_string(case["source"])},
        ruling_text: {_rs_string(case["ruling_text"])},
        card_id: {_rs_string(card["card_id"])},
        card_name: {_rs_string(card["name"])},
        card_kind: CardKind::{card["kind"]},
        level: {_rs_option(card.get("level"), "u8")},
        dp: {_rs_option(card.get("dp"), "i32")},
        play_cost: {card["play_cost"]}u16,
        colors: {_rs_color_list(card["colors"])},
        traits: {_rs_string_list(card["traits"])},
        effect_text: {_rs_string(card.get("effect_text", ""))},
        inherited_text: {_rs_string(card.get("inherited_text", ""))},
        security_text: {_rs_string(card.get("security_text", ""))},
        rule_text: {_rs_string(card.get("rule_text", ""))},
        requirement_text: {_rs_string(card.get("requirement_text", ""))},
        positive_needles: {_rs_string_list(case.get("positive_needles", []))},
        negative_needles: {_rs_string_list(case.get("negative_needles", []))},
    }};"""


def _render_text_scope_test(case: dict[str, Any]) -> str:
    return f"""
#[test]
fn {case["id"]}() {{
{_case_struct(case)}
    run_text_scope_case(&case);
}}
"""


def generate_faq_conformance_module(cases: list[dict[str, Any]]) -> str:
    approved = [case for case in cases if case.get("approved")]
    tests = []
    for case in approved:
        if case["scenario_kind"] == "text_scope_contains":
            tests.append(_render_text_scope_test(case))
        else:
            raise ValueError(f"unsupported scenario kind {case['scenario_kind']}")

    body = """//! Generated rules-FAQ conformance scenarios.
//!
//! Source data: `qa/rules/faq_conformance_cases.json`.
//! Regenerate with:
//! `python code/tools/faq_conformance_generator.py --check` or without
//! `--check` to rewrite this file.

#![allow(clippy::needless_update)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind};

struct TextScopeCase<'a> {
    id: &'a str,
    source: &'a str,
    ruling_text: &'a str,
    card_id: &'a str,
    card_name: &'a str,
    card_kind: CardKind,
    level: Option<u8>,
    dp: Option<i32>,
    play_cost: u16,
    colors: &'a [CardColor],
    traits: &'a [&'a str],
    effect_text: &'a str,
    inherited_text: &'a str,
    security_text: &'a str,
    rule_text: &'a str,
    requirement_text: &'a str,
    positive_needles: &'a [&'a str],
    negative_needles: &'a [&'a str],
}

fn synthetic_card(case: &TextScopeCase<'_>) -> CardData {
    CardData {
        card_id: case.card_id.to_string(),
        card_name: case.card_name.to_string(),
        card_kind: case.card_kind,
        level: case.level,
        dp: case.dp,
        play_cost: case.play_cost,
        colors: case.colors.to_vec(),
        traits: case.traits.iter().map(|trait_name| (*trait_name).to_string()).collect(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: case.effect_text.to_string(),
        inherited_text: case.inherited_text.to_string(),
        security_text: case.security_text.to_string(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: case.card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn text_scope_contains(case: &TextScopeCase<'_>, card: &CardData, needle: &str) -> bool {
    let needle = needle.to_lowercase();
    let haystacks = [
        card.card_name.as_str(),
        card.effect_text.as_str(),
        card.inherited_text.as_str(),
        card.security_text.as_str(),
        case.rule_text,
        case.requirement_text,
    ];
    haystacks
        .iter()
        .any(|field| field.to_lowercase().contains(&needle))
        || card
            .traits
            .iter()
            .any(|trait_name| trait_name.to_lowercase().contains(&needle))
}

fn run_text_scope_case(case: &TextScopeCase<'_>) {
    let mut runner = DebugRunner::builder()
        .add_card(synthetic_card(case))
        .start();
    let _handle = runner.place_on_field(0, case.card_id, Some(0));
    let card = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == case.card_id)
        .expect("generated FAQ card should be present");

    for needle in case.positive_needles {
        assert!(
            text_scope_contains(case, card, needle),
            "FAQ conformance {id} from {source}: expected {card_id} to match \
             `{needle}` under ruling: {ruling}",
            id = case.id,
            source = case.source,
            card_id = case.card_id,
            needle = needle,
            ruling = case.ruling_text
        );
    }

    for needle in case.negative_needles {
        assert!(
            !text_scope_contains(case, card, needle),
            "FAQ conformance {id} from {source}: did not expect {card_id} to \
             match `{needle}` under ruling: {ruling}",
            id = case.id,
            source = case.source,
            card_id = case.card_id,
            needle = needle,
            ruling = case.ruling_text
        );
    }
}
"""
    return body + "".join(tests)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", type=Path, default=CASES_JSON)
    parser.add_argument("--out", type=Path, default=OUTPUT_RS)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)

    rendered = generate_faq_conformance_module(load_cases(args.cases))
    if args.check:
        actual = args.out.read_text(encoding="utf-8")
        if actual != rendered:
            raise SystemExit(f"{args.out} is stale; regenerate FAQ conformance module")
        print(f"{args.out} is current")
        return 0

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(rendered, encoding="utf-8", newline="\n")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
