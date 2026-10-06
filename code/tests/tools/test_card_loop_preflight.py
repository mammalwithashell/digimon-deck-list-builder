"""card-loop new-set preflight (openspec add-card-authoring-loop, task 2.5).

Every check returns `{name, status: ok|warn|fail, detail, remedy}`; the run is
GO iff no check failed, and a NO-GO must stop worker spend. Covers the spec
scenarios "Stale player", "Card without a DCGO script" and "DCGO later adds
the card", plus each check's failure mode, with synthetic fixtures and an
injected node runner (no cargo, no CLIs).
"""
from __future__ import annotations

import json
from pathlib import Path

import pytest

from tools.card_loop import preflight as pf
from tools.card_loop.config import LoopConfig
from tools.card_loop.preflight import (
    Check,
    PreflightNoGo,
    check_dcgo_scripts,
    check_keyword_gate,
    check_mirror_coverage,
    check_oracle_node,
    check_worker_clis,
    dcgo_script_presence,
    dcgo_script_resolution,
    ensure_go,
    newly_available,
    run_preflight,
)

GO_OUTPUT = """GO
  [ok] build: D:\\player\\DCGO.exe (dcgo_commit abc)
  [ok] action_space: matches the engine (0123456789ab)
  [ok] harness_enabled: D:\\h\\harness.enabled present
  [ok] queue: D:\\h has jobs/claimed/done/failed
  [warn] player: not running
        -> `dcgo-harness node up --build <dir>` starts it
"""

STALE_OUTPUT = """NO-GO
  [ok] build: D:\\player\\DCGO.exe (dcgo_commit abc)
  [fail] action_space: player manifest action_space_hash 1111 != engine 2222
        -> this player encodes against a DEAD action space and its recordings would read as engine divergence. Rebuild on the build machine (`dcgo-harness build`) and redistribute the image.
  [ok] harness_enabled: D:\\h\\harness.enabled present
  [ok] queue: D:\\h has jobs/claimed/done/failed
  [warn] player: not running
        -> `dcgo-harness node up --build <dir>` starts it
"""


def _dcgo(tmp_path: Path, *scripts: str) -> Path:
    """A DCGO checkout holding `<SET>/<Colour>/<ID_>.cs` for each `SET/Colour/ID`."""
    root = tmp_path / "DCGO"
    (root / "Assets" / "Scripts" / "CardEffect").mkdir(parents=True)
    for s in scripts:
        set_, colour, cid = s.split("/")
        d = root / "Assets" / "Scripts" / "CardEffect" / set_ / colour
        d.mkdir(parents=True, exist_ok=True)
        (d / f"{cid.replace('-', '_')}.cs").write_text("// script", encoding="utf-8")
    return root


def _statuses(checks) -> dict[str, str]:
    return {c.name: c.status for c in checks}


# --------------------------------------------------------------------------
# check shape + aggregate
# --------------------------------------------------------------------------


def test_check_dict_shape():
    c = Check("x", "fail", "broken", "fix it")
    assert c.to_dict() == {"name": "x", "status": "fail", "detail": "broken", "remedy": "fix it"}


def test_a_fail_without_a_remedy_is_rejected():
    with pytest.raises(ValueError):
        Check("x", "fail", "broken")


def test_unknown_status_is_rejected():
    with pytest.raises(ValueError):
        Check("x", "maybe", "?")


# --------------------------------------------------------------------------
# mirror coverage
# --------------------------------------------------------------------------


def test_mirror_coverage_ok():
    cards = {"BT7-056": {}, "EX7-008": {}}
    official = {"cards": {"BT7-056": {"colors": ["Blue"]}, "EX7-008": {"colors": ["Red"]}}}
    c = check_mirror_coverage(["BT7-056", "EX7-008"], cards, official)
    assert c.status == "ok"


def test_mirror_coverage_fails_on_a_card_missing_from_cards_json():
    official = {"cards": {"BT27-001": {"colors": ["Red"]}}}
    c = check_mirror_coverage(["BT27-001"], {}, official)
    assert c.status == "fail"
    assert "BT27-001" in c.detail
    assert "ingest_cards.py --set BT27" in c.remedy


def test_mirror_coverage_treats_a_colourless_entry_as_a_failed_lookup():
    """card_official entries without `colors` are failed-lookup skeletons, not data."""
    cards = {"P-239": {}}
    official = {"cards": {"P-239": {"card_id": "P-239", "qa": []}}}
    c = check_mirror_coverage(["P-239"], cards, official)
    assert c.status == "fail"
    assert "P-239" in c.detail
    assert "build_card_bundles.py" in c.remedy


def test_mirror_coverage_fails_on_a_card_missing_from_the_official_mirror():
    c = check_mirror_coverage(["BT7-056"], {"BT7-056": {}}, {"cards": {}})
    assert c.status == "fail"
    assert "card_official" in c.detail


# --------------------------------------------------------------------------
# keyword gate
# --------------------------------------------------------------------------


def _card(text: str) -> dict:
    return {"effect_description_eng": text, "inherited_effect_description_eng": "",
            "security_effect_description_eng": ""}


def test_keyword_gate_ok_on_known_keywords():
    cards = {"BT1-001": _card("＜Blocker＞ (When an opponent's Digimon attacks...)")}
    c = check_keyword_gate(["BT1-001"], cards, yaml_ids=set())
    assert c.status == "ok", c.detail


def test_keyword_gate_fails_on_an_unknown_keyword_for_a_card_still_to_implement():
    cards = {"BT27-001": _card("＜Frobnicate 3＞ (does a new thing)")}
    c = check_keyword_gate(["BT27-001"], cards, yaml_ids=set())
    assert c.status == "fail"
    assert "frobnicate" in c.detail and "BT27-001" in c.detail
    assert c.remedy


def test_keyword_gate_downgrades_when_every_affected_card_is_already_authored():
    """A flagged token on an already-authored card is a gate-lexicon gap, not a block."""
    cards = {"BT27-001": _card("＜Frobnicate 3＞ (does a new thing)")}
    c = check_keyword_gate(["BT27-001"], cards, yaml_ids={"BT27-001"})
    assert c.status == "warn"
    assert "frobnicate" in c.detail


def test_keyword_gate_uses_an_injected_triage():
    seen = {}

    class Report:
        flag_for_human = {}
        auto_ingest_subsystem = {}
        auto_ingest = {}

    def triage(texts, set_prefix=""):
        seen["texts"] = list(texts)
        return Report()

    c = check_keyword_gate(["BT1-001"], {"BT1-001": _card("hello")}, yaml_ids=set(), triage=triage)
    assert c.status == "ok"
    assert seen["texts"] and "hello" in seen["texts"][0]


# --------------------------------------------------------------------------
# DCGO script presence
# --------------------------------------------------------------------------


def test_dcgo_presence_maps_each_card(tmp_path):
    root = _dcgo(tmp_path, "BT7/Blue/BT7-056")
    presence = dcgo_script_presence(["BT7-056", "EX7-008"], root)
    assert presence["BT7-056"] == "Assets/Scripts/CardEffect/BT7/Blue/BT7_056.cs"
    assert presence["EX7-008"] is None


def test_card_without_a_dcgo_script_is_a_warn_not_a_fail(tmp_path):
    """Spec: its items are planned `unavailable` and the rest of the run proceeds."""
    root = _dcgo(tmp_path, "BT7/Blue/BT7-056")
    c, presence = check_dcgo_scripts(["BT7-056", "EX7-008"], root)
    assert c.status == "warn"
    assert "EX7-008" in c.detail
    assert presence == {"BT7-056": "Assets/Scripts/CardEffect/BT7/Blue/BT7_056.cs",
                        "EX7-008": None}


def test_all_scripts_present_is_ok(tmp_path):
    root = _dcgo(tmp_path, "BT7/Blue/BT7-056")
    c, _ = check_dcgo_scripts(["BT7-056"], root)
    assert c.status == "ok"


def test_a_missing_dcgo_checkout_fails(tmp_path):
    c, presence = check_dcgo_scripts(["BT7-056"], tmp_path / "nope")
    assert c.status == "fail"
    assert "rule 29" in c.remedy
    assert presence == {"BT7-056": None}


def test_no_dcgo_root_at_all_fails():
    c, _ = check_dcgo_scripts(["BT7-056"], None)
    assert c.status == "fail"


def test_dcgo_later_adds_the_card(tmp_path):
    """Spec: a resume after DCGO gains the script re-enters the card as outstanding."""
    root = _dcgo(tmp_path, "BT7/Blue/BT7-056")
    before = dcgo_script_presence(["BT7-056", "EX7-008"], root)
    _dcgo_add = root / "Assets" / "Scripts" / "CardEffect" / "EX7" / "Red"
    _dcgo_add.mkdir(parents=True)
    (_dcgo_add / "EX7_008.cs").write_text("// new", encoding="utf-8")
    after = dcgo_script_presence(["BT7-056", "EX7-008"], root)
    assert after["EX7-008"] == "Assets/Scripts/CardEffect/EX7/Red/EX7_008.cs"
    assert newly_available(before, after) == ["EX7-008"]


# --- task 9.2: DCGO resolves a card's effects through its CardBaseEntity asset ---


def _cs(root: Path, rel: str, body: str) -> None:
    p = root / "Assets" / "Scripts" / "CardEffect" / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(body, encoding="utf-8", newline="\n")


def _global_class(name: str) -> str:
    return (f"public class {name} : CEntity_Effect\n{{\n"
            "    public override List<ICardEffect> CardEffects(EffectTiming timing, "
            "CardSource card) {}\n}\n")


def _ns_class(ns: str, *names: str) -> str:
    return f"namespace {ns}\n{{\n" + "".join(_global_class(n) for n in names) + "}\n"


def _asset(root: Path, card_id: str, class_name: str, sub: str = "Red/Digimon") -> None:
    """A CardBaseEntity asset (Unity YAML) naming the card's effect class."""
    set_ = card_id.split("-")[0]
    d = root / "Assets" / "CardBaseEntity" / set_ / sub
    d.mkdir(parents=True, exist_ok=True)
    stem = card_id.replace("-", "_")
    (d / f"{stem}.asset").write_text(
        "%YAML 1.1\n--- !u!114 &11400000\nMonoBehaviour:\n"
        f"  m_Name: {stem}\n  CardEffectClassName: {class_name}\n  CardID: {card_id}\n",
        encoding="utf-8", newline="\n")


@pytest.fixture
def asset_dcgo(tmp_path):
    """A DCGO checkout WITH CardBaseEntity assets (the real layout)."""
    root = tmp_path / "DCGO"
    _cs(root, "BT1/Red/BT1_016.cs", _global_class("BT1_016"))   # <Jamming> only
    _cs(root, "BT1/Red/BT1_021.cs", _global_class("BT1_021"))
    _cs(root, "BT1/Red/BT1_040.cs", _global_class("BT1_040"))   # not what the asset runs
    _cs(root, "BT7/Blue/BT7_056.cs", _global_class("BT7_056"))
    _cs(root, "BT25/Yellow/BT25_002.cs",
        _ns_class("DCGO.CardEffects.BT25", "BT25_002", "BT25_003"))
    _cs(root, "BT10/Green/BT10_045.cs", _ns_class("DCGO.CardEffects.BT10", "BT10_045"))
    _cs(root, "EX13/Red/EX13_001.cs", _ns_class("DCGO.CardEffects.EX13", "EX13_001"))
    _cs(root, "BT4/Purple/BT4_038_token.cs", _ns_class("DCGO.CardEffects.Tokens", "BT4_038_token"))
    _asset(root, "BT7-056", "BT7_056")
    _asset(root, "BT1-032", "BT1_016")       # keyword-only reprint -> shared class
    _asset(root, "BT1-040", "BT1_021")       # its own BT1_040.cs is not what DCGO runs
    _asset(root, "BT25-003", "BT25_003")     # class lives in BT25_002.cs
    _asset(root, "BT12-036", "BT10_045")     # namespaced in BT10: DCGO looks in BT12
    _asset(root, "BT3-099", "")              # no class -> EmptyEffectClass
    _asset(root, "P-999", "BT4_038_token")   # token-named class -> Tokens namespace
    return root


def test_asset_naming_a_shared_class_is_available(asset_dcgo):
    """A keyword-only reprint (BT1-032, <Jamming>) has no `BT1_032.cs`; DCGO
    plays it through the class its CardBaseEntity asset names."""
    res = dcgo_script_resolution(["BT1-032", "BT7-056"], asset_dcgo)
    assert res["BT1-032"].path == "Assets/Scripts/CardEffect/BT1/Red/BT1_016.cs"
    assert "BT1_016" in res["BT1-032"].mechanism
    assert res["BT7-056"].path == "Assets/Scripts/CardEffect/BT7/Blue/BT7_056.cs"
    c, presence = check_dcgo_scripts(["BT1-032", "BT7-056"], asset_dcgo)
    assert c.status == "ok"
    assert presence["BT1-032"] == "Assets/Scripts/CardEffect/BT1/Red/BT1_016.cs"
    assert "BT1-032" in c.detail and "CardEffectClassName" in c.detail


def test_the_asset_class_wins_over_a_same_named_script(asset_dcgo):
    assert dcgo_script_presence(["BT1-040"], asset_dcgo) == {
        "BT1-040": "Assets/Scripts/CardEffect/BT1/Red/BT1_021.cs"}


def test_a_class_declared_in_another_file_of_its_namespace_resolves(asset_dcgo):
    # DCGO's BT25_003 (Frimon) is declared inside BT25_002.cs.
    assert dcgo_script_presence(["BT25-003"], asset_dcgo) == {
        "BT25-003": "Assets/Scripts/CardEffect/BT25/Yellow/BT25_002.cs"}


def test_a_class_namespaced_under_another_set_does_not_resolve(asset_dcgo):
    # AddCardEffect looks for `DCGO.CardEffects.BT12.BT10_045`; the class is in
    # `DCGO.CardEffects.BT10`, so DCGO attaches EmptyEffectClass.
    res = dcgo_script_resolution(["BT12-036"], asset_dcgo)
    assert res["BT12-036"].path is None
    assert "EmptyEffectClass" in res["BT12-036"].mechanism


def test_two_classes_in_one_file_both_resolve(asset_dcgo):
    # BT25_002.cs declares BT25_002 AND BT25_003: one file, two cards.
    _asset(asset_dcgo, "BT25-002", "BT25_002")
    assert dcgo_script_presence(["BT25-002", "BT25-003"], asset_dcgo) == {
        "BT25-002": "Assets/Scripts/CardEffect/BT25/Yellow/BT25_002.cs",
        "BT25-003": "Assets/Scripts/CardEffect/BT25/Yellow/BT25_002.cs",
    }


def test_the_whole_tree_is_scanned_at_most_once_per_call(asset_dcgo, monkeypatch):
    # Unresolvable classes (vanilla cards, cross-set namespaces) must not each
    # re-read every CardEffect file.
    for i in range(5):
        _asset(asset_dcgo, f"BT12-{100 + i}", "BT10_045")
    files = len(list((asset_dcgo / "Assets" / "Scripts" / "CardEffect").rglob("*.cs")))
    calls = []
    real = pf._class_declarations
    monkeypatch.setattr(pf, "_class_declarations", lambda p: calls.append(p) or real(p))
    res = dcgo_script_resolution([f"BT12-{100 + i}" for i in range(5)], asset_dcgo)
    assert all(r.path is None for r in res.values())
    assert len(calls) <= files + 5   # one tree scan + one natural-file read per card


def test_an_asset_with_no_class_is_unavailable(asset_dcgo):
    # A vanilla card's asset has an empty `CardEffectClassName:`; the next
    # line ("CardID: ...", "DP: 3000") must not be read as the class name.
    res = dcgo_script_resolution(["BT3-099"], asset_dcgo)
    assert res["BT3-099"].path is None
    assert "names no CardEffectClassName" in res["BT3-099"].mechanism
    assert "EmptyEffectClass" in res["BT3-099"].mechanism


def test_a_token_named_class_resolves_in_the_tokens_namespace(asset_dcgo):
    assert dcgo_script_presence(["P-999"], asset_dcgo) == {
        "P-999": "Assets/Scripts/CardEffect/BT4/Purple/BT4_038_token.cs"}


def test_a_card_with_no_asset_is_unavailable_even_with_a_script(asset_dcgo):
    # EX13: scripts exist, but no CardBaseEntity asset -> not in DCGO's card list.
    res = dcgo_script_resolution(["EX13-001"], asset_dcgo)
    assert res["EX13-001"].path is None
    assert "CardBaseEntity" in res["EX13-001"].mechanism
    c, _ = check_dcgo_scripts(["EX13-001", "BT1-032"], asset_dcgo)
    assert c.status == "warn" and "EX13-001" in c.detail


def test_a_resume_after_dcgo_adds_the_asset_re_enters_the_card(asset_dcgo):
    before = dcgo_script_presence(["EX13-001"], asset_dcgo)
    _asset(asset_dcgo, "EX13-001", "EX13_001")
    after = dcgo_script_presence(["EX13-001"], asset_dcgo)
    assert after["EX13-001"] == "Assets/Scripts/CardEffect/EX13/Red/EX13_001.cs"
    assert newly_available(before, after) == ["EX13-001"]


def test_real_dcgo_resolves_keyword_only_and_shared_class_cards():
    """Pins the task-9.2 finding against the base-repo DCGO (skipped if absent)."""
    root = pf.resolve_dcgo_root()
    if root is None or not (Path(root) / "Assets" / "CardBaseEntity").is_dir():
        pytest.skip("base-repo DCGO not populated")
    res = dcgo_script_resolution(
        ["BT1-032", "ST13-08", "BT14-009", "BT25-003", "BT12-036", "EX13-001"], root)
    assert res["BT1-032"].path.endswith("/BT1_016.cs")      # <Jamming> only
    assert res["ST13-08"].path.endswith("/BT8_071.cs")
    assert res["BT14-009"].path.endswith("/BT9_033.cs")
    assert res["BT25-003"].path.endswith("/BT25_002.cs")
    assert res["BT12-036"].path is None                      # cross-set namespace
    assert res["EX13-001"].path is None                      # no CardBaseEntity asset


# --------------------------------------------------------------------------
# worker CLIs
# --------------------------------------------------------------------------


def test_worker_cli_adapter_not_built_is_a_warn():
    def missing():
        raise ImportError("No module named 'tools.card_loop.workers.claude'")

    checks = check_worker_clis({"claude": missing, "codex": lambda: "codex.exe"})
    assert _statuses(checks) == {"worker_cli.claude": "warn", "worker_cli.codex": "ok"}
    assert "not built" in checks[0].detail


def test_worker_cli_unresolved_binary_fails_with_install_remedy():
    checks = check_worker_clis({"claude": lambda: None, "codex": lambda: "codex.exe"})
    claude = next(c for c in checks if c.name == "worker_cli.claude")
    assert claude.status == "fail"
    assert "install" in claude.remedy.lower()


def test_default_resolvers_degrade_to_warn_when_group3_is_absent(monkeypatch):
    """With the real lazy loader, a missing adapter module is a warn, never a crash."""
    monkeypatch.setattr(pf, "_ADAPTERS", {
        "claude": ("tools.card_loop.workers._no_such_adapter", "resolve_claude_exe"),
        "codex": ("tools.card_loop.workers._no_such_adapter", "resolve_codex_exe"),
    })
    checks = check_worker_clis(pf.load_cli_resolvers())
    assert _statuses(checks) == {"worker_cli.claude": "warn", "worker_cli.codex": "warn"}


# --------------------------------------------------------------------------
# oracle node
# --------------------------------------------------------------------------


def test_oracle_not_configured_is_a_warn():
    checks = check_oracle_node(LoopConfig(), runner=lambda cmd, cwd: pytest.fail("ran"))
    assert _statuses(checks) == {"oracle_node": "warn"}


def test_oracle_node_go_parses_every_check():
    calls = []

    def runner(cmd, cwd):
        calls.append(cmd)
        return 0, GO_OUTPUT

    cfg = LoopConfig(harness_root=r"D:\h", player_dir=r"D:\player")
    checks = check_oracle_node(cfg, runner=runner)
    assert calls == [["cargo", "run", "-q", "-p", "dcgo-harness", "--", "--root", r"D:\h",
                      "node", "status", "--build", r"D:\player"]]
    st = _statuses(checks)
    assert st["oracle_node.action_space"] == "ok"
    assert st["oracle_node.player"] == "warn"
    assert "fail" not in st.values()


def test_oracle_node_omits_root_when_unset():
    calls = []
    check_oracle_node(LoopConfig(player_dir="P"),
                      runner=lambda cmd, cwd: (calls.append(cmd), (0, GO_OUTPUT))[1])
    assert "--root" not in calls[0]
    assert calls[0][-4:] == ["node", "status", "--build", "P"]


def test_stale_player_fails_naming_the_rebuild_remedy():
    """Spec: a player whose action-space hash differs from the engine's -> NO-GO."""
    cfg = LoopConfig(harness_root="H", player_dir="P")
    checks = check_oracle_node(cfg, runner=lambda cmd, cwd: (1, STALE_OUTPUT))
    stale = next(c for c in checks if c.name == "oracle_node.action_space")
    assert stale.status == "fail"
    assert "Rebuild" in stale.remedy


def test_oracle_node_nonzero_exit_without_parseable_checks_fails():
    cfg = LoopConfig(player_dir="P")
    checks = check_oracle_node(cfg, runner=lambda cmd, cwd: (2, "error: needs --root <DIR>"))
    assert _statuses(checks) == {"oracle_node": "fail"}
    assert "harness_root" in checks[0].remedy


def test_oracle_node_runner_crash_fails():
    def boom(cmd, cwd):
        raise FileNotFoundError("cargo")

    checks = check_oracle_node(LoopConfig(player_dir="P"), runner=boom)
    assert _statuses(checks) == {"oracle_node": "fail"}


# --------------------------------------------------------------------------
# run_preflight + the NO-GO contract
# --------------------------------------------------------------------------


@pytest.fixture
def world(tmp_path):
    cards = {"BT7-056": _card(""), "EX7-008": _card("")}
    official = {"cards": {"BT7-056": {"colors": ["Blue"]}, "EX7-008": {"colors": ["Red"]}}}
    return {
        "cards": cards,
        "official": official,
        "dcgo_root": _dcgo(tmp_path, "BT7/Blue/BT7-056"),
        "yaml_ids": set(),
        "cli_resolvers": {"claude": lambda: "claude", "codex": lambda: "codex"},
    }


def test_run_preflight_go_with_warnings(world):
    report = run_preflight(["BT7-056", "EX7-008"], config=LoopConfig(), **world)
    assert report.go is True
    assert report.unavailable == ["EX7-008"]
    st = _statuses(report.checks)
    assert st["dcgo_scripts"] == "warn" and st["oracle_node"] == "warn"
    d = report.to_dict()
    assert d["go"] is True
    assert all(set(c) == {"name", "status", "detail", "remedy"} for c in d["checks"])


def test_stale_player_is_no_go_and_no_worker_is_invoked(world):
    cfg = LoopConfig(harness_root="H", player_dir="P")
    report = run_preflight(["BT7-056"], config=cfg,
                           runner=lambda cmd, cwd: (1, STALE_OUTPUT), **world)
    assert report.go is False

    invoked = []

    def spend():  # stands in for the driver handing a packet to a worker
        ensure_go(report)
        invoked.append("worker")

    with pytest.raises(PreflightNoGo) as e:
        spend()
    assert invoked == []
    assert "action_space" in str(e.value) and "Rebuild" in str(e.value)


def test_ensure_go_passes_on_go(world):
    ensure_go(run_preflight(["BT7-056"], config=LoopConfig(), **world))


def test_format_table_is_ascii(world):
    report = run_preflight(["BT7-056", "EX7-008"], config=LoopConfig(), **world)
    text = report.format_table()
    text.encode("ascii")
    assert text.splitlines()[0].startswith("GO")


# --------------------------------------------------------------------------
# cli_preflight
# --------------------------------------------------------------------------


def _cli_args(tmp_path, world, *extra):
    cards = tmp_path / "cards.json"
    cards.write_text(json.dumps(world["cards"]), encoding="utf-8")
    official = tmp_path / "card_official.json"
    official.write_text(json.dumps(world["official"]), encoding="utf-8")
    yaml_dir = tmp_path / "yaml"
    yaml_dir.mkdir(exist_ok=True)
    return ["--cards-json", str(cards), "--official-json", str(official),
            "--dcgo-root", str(world["dcgo_root"]), "--yaml-dir", str(yaml_dir), *extra]


def test_cli_preflight_exit_0_on_go(tmp_path, world, monkeypatch, capsys):
    monkeypatch.setattr(pf, "load_cli_resolvers", lambda: world["cli_resolvers"])
    rc = pf.cli_preflight(_cli_args(tmp_path, world, "--cards", "BT7-056,EX7-008"))
    out = capsys.readouterr().out
    assert rc == 0
    assert out.startswith("GO")
    assert "dcgo_scripts" in out


def test_cli_preflight_exit_1_on_no_go(tmp_path, world, monkeypatch, capsys):
    monkeypatch.setattr(pf, "load_cli_resolvers",
                        lambda: {"claude": lambda: None, "codex": lambda: None})
    rc = pf.cli_preflight(_cli_args(tmp_path, world, "--cards", "BT7-056"))
    assert rc == 1
    assert capsys.readouterr().out.startswith("NO-GO")


def test_cli_preflight_reads_oracle_settings_from_config(tmp_path, world, monkeypatch):
    monkeypatch.setattr(pf, "load_cli_resolvers", lambda: world["cli_resolvers"])
    calls = []
    monkeypatch.setattr(pf, "default_runner", lambda cmd, cwd: (calls.append(cmd), (1, STALE_OUTPUT))[1])
    cfg = tmp_path / "loop.toml"
    cfg.write_text('player_dir = "P"\nharness_root = "H"\n', encoding="utf-8")
    rc = pf.cli_preflight(_cli_args(tmp_path, world, "--cards", "BT7-056", "--config", str(cfg)))
    assert rc == 1
    assert calls and calls[0][-1] == "P"
