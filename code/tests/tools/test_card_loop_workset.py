"""card-loop work-set resolution (openspec add-card-authoring-loop, group 2).

One test (or more) per scenario of `specs/card-loop-work-set/spec.md`:
explicit cards, decklists -> core by frequency, decklists in any engine
format, archetype through the alias map, set with / without decklists, and
the staple-outranks-one-of ranking rule. Fixtures are synthetic and tiny; one
smoke test resolves a real archetype from `data/deck_library.json`.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from tools.card_loop import workset as ws
from tools.card_loop.config import LoopConfig
from tools.card_loop.workset import (
    WorkSet,
    parse_cards_arg,
    rank_by_decklist_completion,
    resolve_archetype_name,
    resolve_work_set,
)


# --------------------------------------------------------------------------
# fixtures
# --------------------------------------------------------------------------


def _write_library(path: Path, archetypes: dict[str, list[list[str]]]) -> Path:
    """deck_library.json the way it is really stored: `decklist` is a JSON STRING."""
    path.write_text(
        json.dumps(
            {
                "version": 1,
                "archetypes": {
                    name: {
                        "archetype_name": name,
                        "decklists": [
                            {"deck_id": f"{name}-{i}", "decklist": json.dumps(cards)}
                            for i, cards in enumerate(lists)
                        ],
                    }
                    for name, lists in archetypes.items()
                },
            }
        ),
        encoding="utf-8",
    )
    return path


def _write_aliases(path: Path, aliases: dict[str, list[str]]) -> Path:
    path.write_text(json.dumps({"_comment": "test", **aliases}), encoding="utf-8")
    return path


def _text_deck(path: Path, cards: list[str]) -> Path:
    """A decklist in the engine's text format: `<qty> <name> <id>`."""
    lines = [f"1 Card {c}" for c in cards]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return path


@pytest.fixture
def empty_library(tmp_path):
    lib = _write_library(tmp_path / "deck_library.json", {})
    aliases = _write_aliases(tmp_path / "aliases.json", {})
    return {"library_path": lib, "aliases_path": aliases}


@pytest.fixture
def fake_parse(monkeypatch):
    """Route decklist parsing through a recorder that understands `1 Card <ID>`.

    The real engine parser is exercised separately
    (`test_decklists_in_any_engine_format`); this keeps the rest of the suite
    independent of the PyO3 build.
    """
    seen: list[str] = []

    def parse(raw: str) -> list[str]:
        seen.append(raw)
        return [line.split()[-1] for line in raw.splitlines() if line.strip()]

    monkeypatch.setattr(ws, "_engine_parse_deck", lambda: parse)
    return seen


# --------------------------------------------------------------------------
# --cards
# --------------------------------------------------------------------------


def test_cards_arg_accepts_a_comma_list():
    assert parse_cards_arg("BT7-056, ex7-008,BT7-056") == ["BT7-056", "EX7-008"]


def test_cards_arg_accepts_an_at_file(tmp_path):
    f = tmp_path / "ids.txt"
    f.write_text("# core first\nBT7-056\nEX7-008, ST1-03\n\n", encoding="utf-8")
    assert parse_cards_arg(f"@{f}") == ["BT7-056", "EX7-008", "ST1-03"]


def test_cards_arg_rejects_a_malformed_id():
    with pytest.raises(ValueError, match="BT7056"):
        parse_cards_arg("BT7056")


def test_explicit_card_list_is_pool_core_and_ranking_in_given_order(empty_library):
    w = resolve_work_set(cards=["BT7-056", "EX7-008"], **empty_library)
    assert isinstance(w, WorkSet)
    assert w.pool == ["BT7-056", "EX7-008"]
    assert w.core == ["BT7-056", "EX7-008"]
    assert w.ranking == ["BT7-056", "EX7-008"]
    assert w.decklists is None


def test_explicit_order_is_kept_not_sorted(empty_library):
    w = resolve_work_set(cards=["EX7-008", "BT7-056"], **empty_library)
    assert w.ranking == ["EX7-008", "BT7-056"]


def test_no_input_is_an_error(empty_library):
    with pytest.raises(ValueError):
        resolve_work_set(**empty_library)


# --------------------------------------------------------------------------
# --decklists
# --------------------------------------------------------------------------


def test_decklists_define_pool_by_union_and_core_by_frequency(tmp_path, empty_library, fake_parse):
    # Ten lists: A in 7, B in 6, C in all 10, D in 1.  core_fraction 0.7 -> >= 7.
    files = []
    for i in range(10):
        cards = ["BT1-010"]  # C
        if i < 7:
            cards.append("BT1-001")  # A
        if i < 6:
            cards.append("BT1-002")  # B
        if i == 9:
            cards.append("BT1-004")  # D
        files.append(_text_deck(tmp_path / f"deck{i}.txt", cards))

    w = resolve_work_set(decklist_files=files, core_fraction=0.7, **empty_library)
    assert w.pool == ["BT1-001", "BT1-002", "BT1-004", "BT1-010"]
    assert w.core == ["BT1-001", "BT1-010"]
    assert sorted(w.ranking) == w.pool, "ranking is a permutation of the pool"
    assert len(w.decklists) == 10
    assert w.sources["core_threshold"] == 7
    assert w.sources["list_count"] == 10


def test_decklist_files_are_handed_to_the_engine_parser(tmp_path, empty_library, fake_parse):
    raw = "1 Card BT1-001\n1 Card BT1-002\n"
    f = tmp_path / "deck.txt"
    f.write_text(raw, encoding="utf-8")
    resolve_work_set(decklist_files=[f], **empty_library)
    assert fake_parse == [raw], "the raw file text must go to the engine's parse_deck"


def test_decklist_glob_is_expanded(tmp_path, empty_library, fake_parse):
    lists = tmp_path / "lists"
    lists.mkdir()
    _text_deck(lists / "a.txt", ["BT1-001"])
    _text_deck(lists / "b.txt", ["BT1-002"])
    w = resolve_work_set(decklist_files=[str(lists / "*.txt")], **empty_library)
    assert w.pool == ["BT1-001", "BT1-002"]


def test_decklists_in_any_engine_format(tmp_path, empty_library):
    """Real PyO3 binding: TTS JSON and text format both parse, copies preserved."""
    engine = pytest.importorskip("digimon_engine")
    assert hasattr(engine, "parse_deck")
    tts = tmp_path / "tts.json"
    tts.write_text(json.dumps(["BT7-056", "BT7-056", "EX7-008"]), encoding="utf-8")
    text = tmp_path / "text.txt"
    text.write_text("4 Agumon ST1-03\n2 Greymon ST1-07\n", encoding="utf-8")

    w = resolve_work_set(decklist_files=[tts, text], **empty_library)
    assert w.decklists[0] == ["BT7-056", "BT7-056", "EX7-008"]
    assert w.decklists[1] == ["ST1-03"] * 4 + ["ST1-07"] * 2
    assert w.pool == ["BT7-056", "EX7-008", "ST1-03", "ST1-07"]


def test_an_unparseable_decklist_names_the_file(tmp_path, empty_library):
    pytest.importorskip("digimon_engine")
    bad = tmp_path / "bad.txt"
    bad.write_text("this is not a deck", encoding="utf-8")
    with pytest.raises(ValueError, match="bad.txt"):
        resolve_work_set(decklist_files=[bad], **empty_library)


# --------------------------------------------------------------------------
# --archetype
# --------------------------------------------------------------------------


@pytest.fixture
def rocks_library(tmp_path):
    lib = _write_library(
        tmp_path / "deck_library.json",
        {
            "Rocks": [["EX8-005", "EX8-046"], ["EX8-005", "EX8-047"], ["EX8-005"]],
            "Hudiemon": [["BT20-001"]],
        },
    )
    aliases = _write_aliases(tmp_path / "aliases.json", {"Rocks": ["RockClose"]})
    return {"library_path": lib, "aliases_path": aliases}


def test_archetype_resolves_through_the_alias_map(rocks_library):
    w = resolve_work_set(archetype="rockclose", **rocks_library)
    assert w.sources["archetype"] == "Rocks"
    assert w.pool == ["EX8-005", "EX8-046", "EX8-047"]
    assert w.core == ["EX8-005"]  # 3/3 lists; the others are 1/3 < ceil(3*0.7)=3
    assert len(w.decklists) == 3


def test_archetype_canonical_name_is_case_insensitive(rocks_library):
    w = resolve_work_set(archetype="ROCKS", **rocks_library)
    assert w.sources["archetype"] == "Rocks"


def test_unknown_archetype_fails_with_near_miss_suggestions(rocks_library):
    with pytest.raises(LookupError) as e:
        resolve_work_set(archetype="Rokcs", **rocks_library)
    assert "Rocks" in str(e.value)


def test_near_misses_include_aliases(rocks_library):
    library = json.loads(Path(rocks_library["library_path"]).read_text())["archetypes"]
    aliases = json.loads(Path(rocks_library["aliases_path"]).read_text())
    with pytest.raises(LookupError) as e:
        resolve_archetype_name("RockClos", library, aliases)
    assert "RockClose" in str(e.value)


def test_real_archetype_smoke():
    """Real data: the documented alias `RockClose` resolves to `Rocks`."""
    w = resolve_work_set(archetype="RockClose")
    assert w.sources["archetype"] == "Rocks"
    assert w.pool and set(w.core) <= set(w.pool)
    assert sorted(w.ranking) == sorted(w.pool)


# --------------------------------------------------------------------------
# --set
# --------------------------------------------------------------------------


FAKE_CARDS = {
    "BT27-001": {}, "BT27-002": {}, "BT27-003": {},
    "BT2-001": {}, "BT2-002": {},
    "EX7-008": {},
}


def test_set_alone_pool_is_the_set_and_core_the_whole_pool(empty_library):
    w = resolve_work_set(set_prefix="bt27", cards_data=FAKE_CARDS, **empty_library)
    assert w.pool == ["BT27-001", "BT27-002", "BT27-003"]
    assert w.core == w.pool
    assert w.ranking == w.pool  # no known lists -> card id order
    assert w.sources["set"] == "BT27"


def test_set_prefix_is_exact(empty_library):
    """BT2 must not pick up BT27 (author_set.set_resolver anchors on the hyphen)."""
    w = resolve_work_set(set_prefix="BT2", cards_data=FAKE_CARDS, **empty_library)
    assert w.pool == ["BT2-001", "BT2-002"]


def test_unknown_set_names_the_ingest_remedy(empty_library):
    with pytest.raises(ValueError, match="ingest"):
        resolve_work_set(set_prefix="BT99", cards_data=FAKE_CARDS, **empty_library)


def test_set_with_decklists_takes_core_and_ranking_from_the_lists(tmp_path, empty_library, fake_parse):
    # BT27-002 is in every list, BT27-001 in one; EX7-008 is outside the set.
    files = [
        _text_deck(tmp_path / "a.txt", ["BT27-002", "EX7-008"]),
        _text_deck(tmp_path / "b.txt", ["BT27-002", "EX7-008"]),
        _text_deck(tmp_path / "c.txt", ["BT27-002", "BT27-001"]),
    ]
    w = resolve_work_set(set_prefix="BT27", decklist_files=files,
                         cards_data=FAKE_CARDS, **empty_library)
    assert w.pool == ["BT27-001", "BT27-002", "BT27-003"], "pool comes from the set"
    assert w.core == ["BT27-002"], "core comes from the lists, restricted to the pool"
    # BT27-002 alone completes a and b (EX7-008 is out of scope, so done).
    assert w.ranking[0] == "BT27-002"
    assert w.ranking[-1] == "BT27-003", "a set card in no list ranks last"
    assert sorted(w.ranking) == w.pool


def test_set_alone_uses_known_library_lists_for_core(tmp_path):
    lib = _write_library(
        tmp_path / "deck_library.json",
        {"Arch": [["BT27-003", "EX7-008"], ["BT27-003", "BT27-001"]]},
    )
    aliases = _write_aliases(tmp_path / "aliases.json", {})
    w = resolve_work_set(set_prefix="BT27", cards_data=FAKE_CARDS,
                         library_path=lib, aliases_path=aliases)
    assert w.pool == ["BT27-001", "BT27-002", "BT27-003"]
    assert w.core == ["BT27-001", "BT27-003"], "set cards in any known list"
    assert w.ranking[0] == "BT27-003", "completes the first list on its own"
    assert w.ranking[-1] == "BT27-002"
    assert w.sources["known_lists"] == 2


# --------------------------------------------------------------------------
# ranking: greedy decklist completion
# --------------------------------------------------------------------------


def test_a_staple_outranks_a_one_of():
    """Spec scenario: one card in 270 lists, another in 3, both the last missing card."""
    staple, one_of = "BT1-001", "BT1-002"
    lists = [[staple, f"P-{i:03d}"] for i in range(270)] + [[one_of, "P-999"] for _ in range(3)]
    assert rank_by_decklist_completion(lists, {staple, one_of}) == [staple, one_of]


def test_completion_beats_raw_frequency():
    # A and B share three lists, so neither completes anything alone; C completes L4.
    lists = [["A-1", "B-1"], ["A-1", "B-1"], ["A-1", "B-1"], ["C-1"]]
    assert rank_by_decklist_completion(lists, {"A-1", "B-1", "C-1"}) == ["C-1", "A-1", "B-1"]


def test_completion_ties_break_by_list_count():
    # A and B each complete one list; B is in two lists, A in one -> B first.
    lists = [["A-1"], ["B-1"], ["B-1", "Z-1"]]
    assert rank_by_decklist_completion(lists, {"A-1", "B-1", "Z-1"})[:2] == ["B-1", "A-1"]


def test_full_ties_break_by_card_id():
    assert rank_by_decklist_completion([["Y-1"], ["X-1"]], {"X-1", "Y-1"}) == ["X-1", "Y-1"]


def test_completion_counts_are_exposed():
    order = ws.decklist_completion_order([["A-1"], ["A-1"], ["B-1", "A-1"]], {"A-1", "B-1"})
    assert order == [("A-1", 2), ("B-1", 1)]


def test_cards_not_outstanding_count_as_done():
    # D-1 is done, so the list [C-1, D-1] is completed by C-1 alone.
    lists = [["C-1", "D-1"], ["E-1", "F-1"], ["E-1", "F-1"]]
    assert rank_by_decklist_completion(lists, {"C-1", "E-1", "F-1"})[0] == "C-1"


def test_outstanding_cards_in_no_list_rank_last_by_id():
    order = rank_by_decklist_completion([["A-1"]], {"A-1", "Q-2", "Q-1"})
    assert order == ["A-1", "Q-1", "Q-2"]


def test_copies_within_a_list_do_not_inflate_counts():
    lists = [["A-1", "A-1", "A-1", "A-1"], ["B-1"], ["B-1"]]
    assert rank_by_decklist_completion(lists, {"A-1", "B-1"}) == ["B-1", "A-1"]


# --------------------------------------------------------------------------
# plan (task 2.6)
# --------------------------------------------------------------------------


def _plan_args(tmp_path, *extra: str) -> list[str]:
    lib = _write_library(tmp_path / "deck_library.json", {})
    aliases = _write_aliases(tmp_path / "aliases.json", {})
    cards = tmp_path / "cards.json"
    cards.write_text(json.dumps({"BT7-056": {"card_id": "BT7-056"},
                                 "EX7-008": {"card_id": "EX7-008"}}), encoding="utf-8")
    official = tmp_path / "card_official.json"
    official.write_text(json.dumps({"cards": {"BT7-056": {"colors": ["Blue"]},
                                              "EX7-008": {"colors": ["Red"]}}}),
                        encoding="utf-8")
    dcgo = tmp_path / "DCGO"
    (dcgo / "Assets" / "Scripts" / "CardEffect" / "BT7" / "Blue").mkdir(parents=True)
    (dcgo / "Assets" / "Scripts" / "CardEffect" / "BT7" / "Blue" / "BT7_056.cs").write_text("//")
    yaml_dir = tmp_path / "yaml"
    yaml_dir.mkdir()
    return [
        "--library", str(lib), "--aliases", str(aliases),
        "--cards-json", str(cards), "--official-json", str(official),
        "--dcgo-root", str(dcgo), "--yaml-dir", str(yaml_dir),
        "--runs-dir", str(tmp_path / "runs"), *extra,
    ]


@pytest.fixture
def quiet_clis(monkeypatch):
    """Both worker CLIs 'resolve' so preflight is GO apart from what a test changes."""
    from tools.card_loop import preflight

    monkeypatch.setattr(preflight, "load_cli_resolvers",
                        lambda: {"claude": lambda: "claude.exe", "codex": lambda: "codex.exe"})


def test_plan_writes_a_frozen_plan(tmp_path, quiet_clis):
    args = _plan_args(tmp_path, "--cards", "BT7-056,EX7-008", "--run-id", "r1")
    assert ws.cli_plan(args) == 0
    plan_path = tmp_path / "runs" / "r1" / "plan.json"
    plan = json.loads(plan_path.read_text(encoding="utf-8"))

    assert plan["run_id"] == "r1"
    assert plan["inputs"]["cards"] == ["BT7-056", "EX7-008"]
    assert plan["work_set"]["pool"] == ["BT7-056", "EX7-008"]
    assert plan["work_set"]["core"] == ["BT7-056", "EX7-008"]
    assert plan["work_set"]["ranking"] == ["BT7-056", "EX7-008"]
    # per-card DCGO presence: EX7-008 has no script -> unavailable
    assert plan["dcgo"]["scripts"]["BT7-056"].endswith("BT7_056.cs")
    assert plan["dcgo"]["scripts"]["EX7-008"] is None
    assert plan["dcgo"]["unavailable"] == ["EX7-008"]
    # preflight recorded, GO (no oracle configured is only a warn)
    assert plan["preflight"]["go"] is True
    assert {c["name"] for c in plan["preflight"]["checks"]} >= {
        "mirror_coverage", "keyword_gate", "dcgo_scripts", "worker_cli.claude",
        "worker_cli.codex", "oracle_node"}
    # config hash = sha256 of the json-dumped effective config, sorted keys
    expected = hashlib.sha256(
        json.dumps(plan["config"], sort_keys=True).encode("utf-8")).hexdigest()
    assert plan["config_sha256"] == expected


def test_plan_config_hash_matches_loop_config(tmp_path, quiet_clis):
    from dataclasses import asdict

    args = _plan_args(tmp_path, "--cards", "BT7-056", "--run-id", "r1")
    assert ws.cli_plan(args) == 0
    plan = json.loads((tmp_path / "runs" / "r1" / "plan.json").read_text(encoding="utf-8"))
    assert plan["config_sha256"] == ws.config_sha256(LoopConfig())
    assert plan["config"] == json.loads(json.dumps(asdict(LoopConfig())))


def test_plan_refuses_to_overwrite_a_frozen_plan(tmp_path, quiet_clis, capsys):
    args = _plan_args(tmp_path, "--cards", "BT7-056", "--run-id", "r1")
    assert ws.cli_plan(args) == 0
    assert ws.cli_plan(args) == 2
    assert "already exists" in capsys.readouterr().err


def test_plan_on_no_go_records_preflight_and_exits_1(tmp_path, monkeypatch):
    from tools.card_loop import preflight

    monkeypatch.setattr(preflight, "load_cli_resolvers",
                        lambda: {"claude": lambda: None, "codex": lambda: "codex.exe"})
    args = _plan_args(tmp_path, "--cards", "BT7-056", "--run-id", "r1")
    assert ws.cli_plan(args) == 1
    plan = json.loads((tmp_path / "runs" / "r1" / "plan.json").read_text(encoding="utf-8"))
    assert plan["preflight"]["go"] is False
    failing = [c for c in plan["preflight"]["checks"] if c["status"] == "fail"]
    assert [c["name"] for c in failing] == ["worker_cli.claude"]
    assert failing[0]["remedy"]


def test_plan_skip_preflight_still_records_dcgo_presence(tmp_path):
    args = _plan_args(tmp_path, "--cards", "BT7-056,EX7-008", "--run-id", "r1",
                      "--skip-preflight")
    assert ws.cli_plan(args) == 0
    plan = json.loads((tmp_path / "runs" / "r1" / "plan.json").read_text(encoding="utf-8"))
    assert plan["preflight"] == {"skipped": True}
    assert plan["dcgo"]["unavailable"] == ["EX7-008"]


def test_plan_core_fraction_flag_overrides_config(tmp_path, quiet_clis, fake_parse):
    decks = [_text_deck(tmp_path / f"d{i}.txt", ["BT7-056"] + (["EX7-008"] if i < 2 else []))
             for i in range(4)]
    args = _plan_args(tmp_path, "--decklists", *map(str, decks), "--core-fraction", "0.5",
                      "--run-id", "r1")
    assert ws.cli_plan(args) == 0
    plan = json.loads((tmp_path / "runs" / "r1" / "plan.json").read_text(encoding="utf-8"))
    assert plan["inputs"]["core_fraction"] == 0.5
    assert plan["work_set"]["core"] == ["BT7-056", "EX7-008"]  # 2/4 >= ceil(4*0.5)


def test_plan_bad_input_is_a_usage_error(tmp_path, quiet_clis, capsys):
    args = _plan_args(tmp_path, "--archetype", "Nope", "--run-id", "r1")
    assert ws.cli_plan(args) == 2
    assert "Nope" in capsys.readouterr().err


def test_dispatcher_routes_plan(tmp_path, quiet_clis):
    from tools.card_loop import __main__ as cli

    args = _plan_args(tmp_path, "--cards", "BT7-056", "--run-id", "r1")
    assert cli.main(["plan", *args]) == 0
    assert (tmp_path / "runs" / "r1" / "plan.json").is_file()


# --------------------------------------------------------------------------
# task 2.4: the campaign planner takes a resolved work set
# --------------------------------------------------------------------------


@pytest.fixture
def campaign_workspace(tmp_path):
    """Specs for EX12-004 and BT7-056 (real clause text via exam_binding);
    BT8-084 has no spec, so it lands in `implement`."""
    cards = tmp_path / "specs"
    for set_dir, cid in (("ex12", "EX12-004"), ("bt7", "BT7-056")):
        (cards / set_dir).mkdir(parents=True, exist_ok=True)
        (cards / set_dir / f"{cid}.yaml").write_text(f"card: {cid}\n", encoding="utf-8")
    (tmp_path / "scenarios").mkdir()
    (tmp_path / "verdicts").mkdir()
    return {"cards_dir": cards, "scenarios_dir": tmp_path / "scenarios",
            "verdicts_path": tmp_path / "verdicts"}


def test_campaign_plans_from_a_resolved_work_set(campaign_workspace, empty_library):
    from tools.clause_coverage.campaign import build_plan_for_pool

    w = resolve_work_set(cards=["BT8-084", "BT7-056", "EX12-004"], **empty_library)
    w.core = ["EX12-004"]  # a work set whose core is a strict subset of the pool
    plan = build_plan_for_pool(w.pool, w.core, ranking=w.ranking, label="r1",
                               **campaign_workspace)
    assert plan["implement"] == ["BT8-084"]
    assert plan["core"] == {"cards": ["EX12-004"], "threshold": None,
                            "list_count": None, "fraction": None}
    ids = [c["clause_id"] for c in plan["exam"]]
    assert ids == ["EX12-004#inherited#0", "BT7-056#effect#0", "BT7-056#inherited#0"]
    assert plan["label"] == "r1" and plan["archetype"] is None


def test_campaign_ranking_orders_clauses_within_the_tail(campaign_workspace):
    from tools.clause_coverage.campaign import build_plan_for_pool

    ranked = build_plan_for_pool(["BT7-056", "EX12-004"], [],
                                 ranking=["EX12-004", "BT7-056"], **campaign_workspace)
    assert [c["card_id"] for c in ranked["exam"]] == ["EX12-004", "BT7-056", "BT7-056"]
    unranked = build_plan_for_pool(["BT7-056", "EX12-004"], [], **campaign_workspace)
    assert [c["card_id"] for c in unranked["exam"]] == ["BT7-056", "BT7-056", "EX12-004"]


def test_campaign_archetype_wrapper_matches_the_pool_planner(tmp_path, campaign_workspace):
    from tools.clause_coverage import archetype as archetype_mod
    from tools.clause_coverage.campaign import build_plan, build_plan_for_pool

    lib = _write_library(tmp_path / "deck_library.json",
                         {"Fixtures": [["EX12-004", "BT8-084"], ["EX12-004"]]})
    by_archetype = build_plan("Fixtures", library_path=lib, **campaign_workspace)
    entry = archetype_mod.load_archetypes(lib)["Fixtures"]
    by_pool = build_plan_for_pool(archetype_mod.pool(entry), archetype_mod.core(entry),
                                  label="Fixtures", **campaign_workspace)
    assert by_archetype["archetype"] == "Fixtures"
    for key in ("pool", "core", "implement", "exam", "skipped", "denominator"):
        assert by_archetype[key] == by_pool[key], key


def test_campaign_cli_reads_a_card_loop_plan(tmp_path, campaign_workspace, capsys):
    from tools.clause_coverage import campaign

    args = _plan_args(tmp_path, "--cards", "BT7-056,EX12-004,BT8-084", "--run-id", "r9",
                      "--skip-preflight")
    assert ws.cli_plan(args) == 0
    capsys.readouterr()
    plan_json = tmp_path / "runs" / "r9" / "plan.json"
    rc = campaign.main(["--work-set", str(plan_json),
                        "--cards-dir", str(campaign_workspace["cards_dir"]),
                        "--scenarios-dir", str(campaign_workspace["scenarios_dir"]),
                        "--verdicts", str(campaign_workspace["verdicts_path"]), "--json"])
    assert rc == 0
    plan = json.loads(capsys.readouterr().out)
    assert plan["label"] == "r9"
    assert plan["pool"] == ["BT7-056", "EX12-004", "BT8-084"]
    assert plan["implement"] == ["BT8-084"]


def test_campaign_cli_archetype_flag_still_works(tmp_path, campaign_workspace, capsys):
    from tools.clause_coverage import campaign

    lib = _write_library(tmp_path / "deck_library.json",
                         {"Fixtures": [["EX12-004", "BT8-084"], ["EX12-004"]]})
    rc = campaign.main(["--archetype", "Fixtures", "--library", str(lib),
                        "--cards-dir", str(campaign_workspace["cards_dir"]),
                        "--scenarios-dir", str(campaign_workspace["scenarios_dir"]),
                        "--verdicts", str(campaign_workspace["verdicts_path"])])
    assert rc == 0
    out = capsys.readouterr().out
    assert out.startswith("Fixtures")
    assert ">=2 of 2 lists" in out
