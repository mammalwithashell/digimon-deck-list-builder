from pathlib import Path

from tools.faq_conformance_generator import (
    CASES_JSON,
    OUTPUT_RS,
    generate_faq_conformance_module,
    load_cases,
)


def test_curated_cases_seed_store_champs_and_ex12_rulings():
    cases = load_cases(CASES_JSON)
    approved = [case for case in cases if case["approved"]]
    sources = {case["source"] for case in approved}

    assert any("BT25-" in source for source in sources)
    assert sum("EX12-" in source for source in sources) >= 2
    assert all(case["ruling_text"].strip() for case in approved)
    assert all(case["scenario_kind"] for case in approved)


def test_generated_faq_conformance_module_is_current():
    rendered = generate_faq_conformance_module(load_cases(CASES_JSON))
    actual = Path(OUTPUT_RS).read_text(encoding="utf-8")
    assert actual == rendered
