"""Tests for the official Q&A scraper (tools.scrape_official_qa).

The fixtures mirror the live world.digimoncard.com markup (2026-10-04): one
`cardInfoBox` per printing, each holding a `cardFaqList` of `cardFaqListItem`s
with a Q-number, an optional date, a question and an answer. The old parser in
build_card_bundles.py matched `cardInfoTit … </dt> <dd> … </dd>` lazily, which
swallowed the first question into the *title* and kept only the first answer.
"""

from tools.scrape_official_qa import (
    finalize,
    merge_page,
    new_dataset,
    parse_date,
    parse_qa_page,
)


def _item(q_id, question, answer, date=None):
    date_html = f'<p class="cardFaqDate">{date}</p>' if date else ""
    return f"""
    <li class="cardFaqListItem">
      <div class="cardFaqNumCol">
        <p class="cardFaqNum">{q_id}</p>
        {date_html}
      </div>
      <dl class="cardFaqTxtCol">
        <dt class="cardFaqQuestion">
          {question}                    </dt>
        <dd class="cardFaqAnswer">
          {answer}
                              </dd>
      </dl>
    </li>"""


def _printing(card_id, items):
    faq = ""
    if items:
        faq = f"""
    <div class="cardInfoBox">
      <p class="cardInfoTit">Card Q&A</p>
      <ul class="cardFaqList">{''.join(items)}
      </ul>
    </div>"""
    return f"""
    <div class="cardTitleCol"><ul class="cardTitleList">
      <li class="cardNo">
        {card_id}            </li>
      <li class="cardRarity">R</li>
    </ul></div>
    <div class="cardInfoBox"><p class="cardInfoTit">Effect</p>
      <p class="cardInfoData">[On Play] Reveal the top 3 cards of your deck.</p></div>
    {faq}"""


Q1 = _item("Q1601", "If I reveal only a card with [X Antibody], can I add it?",
           "Yes. You still add that card to your hand.", "Mar. 28, 2024")
Q2 = _item("Q1602", "Does this card&#039;s inherited effect activate?",
           "No, it does <b>not</b> activate.<br>It isn&rsquo;t placed by an effect.")


def _page(*printings):
    return "<html><body>" + "".join(printings) + "</body></html>"


def test_every_entry_in_a_block_is_kept_with_its_question():
    result = parse_qa_page("BT7-056", _page(_printing("BT7-056", [Q1, Q2])))
    assert result.found
    assert [e["q_id"] for e in result.entries] == ["Q1601", "Q1602"]
    assert result.entries[0]["question"] == (
        "If I reveal only a card with [X Antibody], can I add it?"
    )
    assert result.entries[0]["answer"] == "Yes. You still add that card to your hand."


def test_entries_repeated_per_printing_are_deduplicated_by_q_number():
    page = _page(_printing("BT7-056", [Q1, Q2]), _printing("BT7-056", [Q1, Q2]))
    assert [e["q_id"] for e in parse_qa_page("BT7-056", page).entries] == ["Q1601", "Q1602"]


def test_markup_is_stripped_and_entities_unescaped():
    entry = parse_qa_page("BT7-056", _page(_printing("BT7-056", [Q2]))).entries[0]
    assert entry["question"] == "Does this card's inherited effect activate?"
    assert entry["answer"] == "No, it does not activate. It isn’t placed by an effect."


def test_date_is_iso_when_present_and_none_when_absent():
    entries = parse_qa_page("BT7-056", _page(_printing("BT7-056", [Q1, Q2]))).entries
    assert entries[0]["date"] == "2024-03-28"
    assert entries[1]["date"] is None


def test_parse_date_handles_full_and_irregular_month_abbreviations():
    assert parse_date("Mar. 28, 2024") == "2024-03-28"
    assert parse_date("June 3, 2025") == "2025-06-03"
    assert parse_date("Sept. 3, 2025") == "2025-09-03"
    assert parse_date("not a date") == "not a date"  # kept raw, never dropped


def test_card_found_without_qa_has_no_entries():
    result = parse_qa_page("ST1-03", _page(_printing("ST1-03", [])))
    assert result.found
    assert result.entries == []


def test_no_results_page_is_not_found():
    # A search that finds nothing must not be recorded as "this card has no Q&A".
    result = parse_qa_page("TEST-020", "<html><body><p>Card List</p></body></html>")
    assert not result.found


def test_page_for_a_different_card_number_is_not_found():
    result = parse_qa_page("ST1-01", _page(_printing("ST1-010", [Q1])))
    assert not result.found


def test_only_the_requested_cards_printings_contribute_rulings():
    q9 = _item("Q9001", "A ruling about another card?", "Not this card's.")
    page = _page(_printing("ST1-01", [Q1]), _printing("ST1-010", [q9]))
    assert [e["q_id"] for e in parse_qa_page("ST1-01", page).entries] == ["Q1601"]


def test_dataset_links_a_shared_ruling_to_every_card_that_prints_it():
    ds = new_dataset()
    merge_page(ds, "BT7-056", parse_qa_page("BT7-056", _page(_printing("BT7-056", [Q1, Q2]))))
    merge_page(ds, "RB1-020", parse_qa_page("RB1-020", _page(_printing("RB1-020", [Q1]))))
    out = finalize(ds)
    assert out["cards"] == {"BT7-056": ["Q1601", "Q1602"], "RB1-020": ["Q1601"]}
    assert out["qa"]["Q1601"]["card_ids"] == ["BT7-056", "RB1-020"]
    assert out["qa"]["Q1602"]["card_ids"] == ["BT7-056"]


def test_rescraping_a_card_replaces_its_rulings_and_drops_orphans():
    ds = new_dataset()
    merge_page(ds, "BT7-056", parse_qa_page("BT7-056", _page(_printing("BT7-056", [Q1, Q2]))))
    merge_page(ds, "BT7-056", parse_qa_page("BT7-056", _page(_printing("BT7-056", [Q1]))))
    out = finalize(ds)
    assert out["cards"]["BT7-056"] == ["Q1601"]
    assert "Q1602" not in out["qa"]


def test_conflicting_text_for_one_q_number_is_recorded_not_silently_overwritten():
    other = _item("Q1601", "A different question?", "A different answer.")
    ds = new_dataset()
    merge_page(ds, "BT7-056", parse_qa_page("BT7-056", _page(_printing("BT7-056", [Q1]))))
    merge_page(ds, "RB1-020", parse_qa_page("RB1-020", _page(_printing("RB1-020", [other]))))
    out = finalize(ds)
    assert out["qa"]["Q1601"]["answer"] == "Yes. You still add that card to your hand."
    assert out["conflicts"] == [{"q_id": "Q1601", "card_id": "RB1-020"}]


def test_not_found_cards_go_to_failed_and_keep_no_card_entry():
    ds = new_dataset()
    merge_page(ds, "TEST-020", parse_qa_page("TEST-020", "<html></html>"))
    out = finalize(ds)
    assert out["failed"] == ["TEST-020"]
    assert "TEST-020" not in out["cards"]


def test_q_numbers_sort_numerically_for_stable_diffs():
    ds = new_dataset()
    q601 = _item("Q601", "Is this card counted?", "Yes.")
    merge_page(ds, "ST1-01", parse_qa_page("ST1-01", _page(_printing("ST1-01", [Q1, q601]))))
    out = finalize(ds)
    assert list(out["qa"]) == ["Q601", "Q1601"]
    assert out["cards"]["ST1-01"] == ["Q601", "Q1601"]
