"""Content identity, source provenance and partition safety for the pool refresh."""
import json

import pytest

from tools.cloud_search.prepare_ex13_pool import (
    assign_partitions, balanced_runtime_slots, choose_diverse, combine_diagnostic_pools,
    current_compatible_build, fingerprint, native_extension_hashes, parse_digilab_page, qualification_reasons, require_new_output, select_partitions,
)


def candidate(key, *, event=None, cards=None, seen=False, field=32, place=1, when="2026-10-24"):
    cards = cards or [f"BT25-{key:03d}"] * 4
    return dict(source_deck_id=str(key), cards=cards, fingerprint=fingerprint(cards),
                event_url=event or f"https://example.test/event/{key}", field_size=field,
                placement=place, event_date=when, format="EX13", record=dict(wins=4, losses=0, draws=0),
                seen_in_previous_training=seen)


def test_native_extension_fingerprints_cover_windows_and_linux(tmp_path):
    windows = tmp_path / "_engine.cp313-win_amd64.pyd"
    linux = tmp_path / "_engine.cpython-313-x86_64-linux-gnu.so"
    windows.write_bytes(b"windows extension")
    linux.write_bytes(b"linux extension")
    (tmp_path/"__init__.py").write_text("python wrapper")
    hashes = native_extension_hashes(tmp_path)
    assert set(hashes) == {str(windows),str(linux)}
    assert len(set(hashes.values())) == 2
    assert all(len(value) == 64 for value in hashes.values())


def test_fingerprint_ignores_order_but_keeps_every_copy_and_egg():
    assert fingerprint(["EX13-001", "BT1-001", "BT1-001"]) == fingerprint(["BT1-001", "EX13-001", "BT1-001"])
    assert fingerprint(["BT1-001", "EX13-001"]) != fingerprint(["BT1-001", "BT1-001", "EX13-001"])
    assert fingerprint(["BT1-001", "EX13-001"]) != fingerprint(["BT1-001", "EX13-002"])


def test_qualification_does_not_accept_tiny_field_or_losing_record():
    assert "field_below_8_or_unmeasured" in qualification_reasons(candidate(1, field=4))
    row = candidate(1)
    row["record"] = dict(wins=1, losses=3, draws=0)
    assert "no_nonlosing_two_win_record" in qualification_reasons(row)
    row = candidate(1, field=48, place=43)
    assert "outside_top_third_or_unmeasured" in qualification_reasons(row)


def test_missing_record_is_preserved_and_penalized_in_ranking():
    row, measured = candidate(1), candidate(2)
    row["record"] = None
    assert qualification_reasons(row) == []
    assert choose_diverse([row,measured], count=1)[0] == measured
    assert row["record"] is None


@pytest.mark.parametrize("field,place,valid", [(11,4,True),(11,5,False),(8,1,True),(8,4,False)])
def test_qualification_top_third_is_explicitly_rounded_up(field, place, valid):
    assert (not qualification_reasons(candidate(1, field=field, place=place))) == valid


def test_mixed_format_and_old_results_are_not_current_ex13_evidence():
    old = candidate(1, when="2026-09-30")
    mixed = candidate(2)
    mixed["format"] = "BT26/EX13"
    assert "outside_October_window" in qualification_reasons(old)
    assert "not_explicit_EX13" in qualification_reasons(mixed)


def test_reviewed_current_global_build_is_usable_without_tournament_format_claim():
    row = candidate(1)
    row["format"] = "BT26/EX13"
    assert not current_compatible_build(row)
    row["ex13_compatible_reviewed"] = True
    assert current_compatible_build(row)
    assert "not_explicit_EX13" in qualification_reasons(row)
    row["event_date"] = "2026-09-15"
    assert not current_compatible_build(row)


@pytest.mark.parametrize("when", [None, "", "2026-10-02junk", "not-a-date", "2026-10-99"])
def test_invalid_date_stays_unmeasured_and_cannot_enter_lower_evidence_selection(when):
    row = candidate(1)
    row["event_date"] = when
    assert "event_date_unmeasured" in qualification_reasons(row)
    assert not current_compatible_build(row)


def test_selection_deduplicates_identical_counts_across_events():
    first, duplicate = candidate(1), candidate(2, cards=candidate(1)["cards"])
    chosen = choose_diverse([first, duplicate, candidate(3), candidate(4)])
    assert len(chosen) == 3
    assert len({row["fingerprint"] for row in chosen}) == 3


def test_independent_event_wins_tie_and_input_order_does_not_change_choice():
    rows = [candidate(1, event="same"), candidate(2, event="same"), candidate(3, event="other")]
    chosen = choose_diverse(rows, count=2)
    assert len({row["event_url"] for row in chosen}) == 2
    assert chosen == choose_diverse(list(reversed(rows)), count=2)


def test_newest_small_event_does_not_automatically_beat_recent_large_finish():
    rows = [candidate(1, field=8, when="2026-10-26"), candidate(2, field=64, when="2026-10-22")]
    assert choose_diverse(rows, count=1)[0]["source_deck_id"] == "2"


def test_partition_shortfall_does_not_duplicate_or_label_training_as_heldout():
    for count in range(1,5):
        training, heldout = assign_partitions([candidate(i) for i in range(count)])
        assert len(training) == min(count,2)
        assert len(heldout) == max(count-2,0)
        assert {r["fingerprint"] for r in training}.isdisjoint(r["fingerprint"] for r in heldout)


def test_previous_training_never_becomes_unseen_evaluation():
    rows = [candidate(i, seen=i<4) for i in range(6)]
    training, heldout = select_partitions(rows)
    assert len(training) == len(heldout) == 2
    assert all(not row["seen_in_previous_training"] for row in heldout)
    assert {r["fingerprint"] for r in training}.isdisjoint(r["fingerprint"] for r in heldout)
    _, missing = select_partitions(rows[:4])
    assert missing == []


def test_runtime_padding_preserves_equal_archetype_probability_without_new_lists():
    rows = [dict(archetype="A",deck_id="sha256:a",cards=["BT1-001"]),
            dict(archetype="A",deck_id="sha256:b",cards=["BT1-002"]),
            dict(archetype="B",deck_id="sha256:c",cards=["BT1-003"])]
    slots = balanced_runtime_slots(rows)
    assert len(slots) == 4
    assert sum(row["name"].startswith("A:") for row in slots) == 2
    assert sum(row["name"].startswith("B:") for row in slots) == 2
    assert len({fingerprint(row["cards"]) for row in slots}) == 3
    assert len({row["name"] for row in slots}) == 4
    assert len(rows) == 3


def test_runtime_padding_rejects_duplicate_contents_in_distinct_manifest():
    rows = [dict(archetype="A",deck_id="sha256:a",cards=["BT1-001"])]*2
    with pytest.raises(ValueError,match="distinct"):
        balanced_runtime_slots(rows)


@pytest.mark.parametrize("artifact", ["train_snapshot.json","heldout_snapshot.json","decks.json","pet_regression_snapshot.json"])
def test_preparation_refuses_complete_or_partial_output(tmp_path, artifact):
    (tmp_path/artifact).write_text("existing data")
    with pytest.raises(FileExistsError,match="Preserving"):
        require_new_output(tmp_path)
    assert (tmp_path/artifact).read_text() == "existing data"


def test_source_inputs_do_not_prevent_new_preparation(tmp_path):
    (tmp_path/"source_candidates.json").write_text("[]")
    require_new_output(tmp_path)


def test_combined_panel_uses_unseen_lists_and_labels_only_missing_pet_fallbacks():
    heldout = {"Meta":["new1","new2"],"PetWithNewList":["new3"]}
    regression = {"PetWithNewList":["old1"],"Light Hybrid":["old2"],"ShineGreymon Data Squad":["old3"]}
    combined, fallbacks = combine_diagnostic_pools(heldout,regression,["Meta","PetWithNewList","Light Hybrid","ShineGreymon Data Squad"])
    assert combined["PetWithNewList"] == ["new3"]
    assert fallbacks == ["Light Hybrid","ShineGreymon Data Squad"]
    assert set(combined) == set(heldout)|set(regression)
    assert regression["PetWithNewList"] == ["old1"]


def test_combined_panel_refuses_silent_archetype_drop():
    with pytest.raises(ValueError,match="No diagnostic list"):
        combine_diagnostic_pools({"Known":[1]}, {}, ["Known","Missing"])


def page_html(slug="beelstarmon-3m"):
    work = dict(**{"@type":"CreativeWork"}, dateCreated="2026-10-24", author={"name":"Pilot"})
    trail = {"@type":"BreadcrumbList", "itemListElement":[
        {"name":"Beelstarmon (3M)", "item":f"https://digimon.digilab.cards/deck/{slug}"},
        {"name":"Store Championship", "item":"https://digimon.digilab.cards/tournament/11259"}]}
    stats = {"Record":"6-0-0", "Place":"1st", "Players":"42", "Rounds":"6", "Format":"EX13"}
    return (''.join(f'<script type="application/ld+json">{json.dumps(row)}</script>' for row in (work,trail)) +
            ''.join(f'<div class="decklist-stat-cell"><div class="stat-label">{k}</div><div class="stat-value">{v}</div></div>' for k,v in stats.items()) +
            '<a class="card-cell-open" href="/card/BT25-005"><span class="card-count-badge">×4</span></a>' +
            '<a class="card-cell-open" href="/card/BT26-083"><span class="card-count-badge">×2</span></a>').encode()


def test_parser_preserves_exact_counts_event_and_record():
    row = parse_digilab_page(page_html(), "https://digimon.digilab.cards/decklist/126919", "Three Musketeers")
    assert row["card_counts"] == {"BT25-005":4, "BT26-083":2}
    assert row["field_size"] == 42 and row["placement"] == 1
    assert row["record"] == {"wins":6, "losses":0, "draws":0}
    assert row["event_date"] == "2026-10-24"
    assert row["source_deck_id"] == "digilab_result_126919"


def test_parser_rejects_wrong_archetype_and_duplicate_count_rows():
    with pytest.raises(ValueError, match="archetype mismatch"):
        parse_digilab_page(page_html("data-squad-ravemon"), "https://digimon.digilab.cards/decklist/1", "Three Musketeers")
    repeated = page_html() + b'<a class="card-cell-open" href="/card/BT25-005"><span class="card-count-badge">x4</span></a>'
    with pytest.raises(ValueError, match="repeated count"):
        parse_digilab_page(repeated, "https://digimon.digilab.cards/decklist/1", "Three Musketeers")
