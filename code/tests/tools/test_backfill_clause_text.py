import pytest

from tools.backfill_clause_text import annotate, printed_spans


def spec(effects):
    return "card: X-1\nname: X\nkind: digimon\ncolor: [red]\ncost: 1\neffects:\n" + effects


def section(text, label="Effect"):
    return [{"label": label, "text": text}]


def test_backfill_preserves_comments_lines_and_exact_printed_span():
    raw = spec('  # keep this\n  - when: on_play # and this\n    process: []\n')
    updated, count, total = annotate(raw, section('[On Play] You may draw 1.'))
    assert (count, total) == (1, 1)
    assert updated == raw.replace('  - when:', '  - text: "[On Play] You may draw 1."\n    when:')
    assert annotate(updated, section('[On Play] You may draw 1.')) == (updated, 0, 1)


def test_backfill_refuses_ambiguity_in_both_directions():
    raw = spec('  - when: on_play\n  - when: on_play\n')
    assert annotate(raw, section('[On Play] Draw 1.')) == (raw, 0, 2)
    raw = spec('  - when: on_play\n')
    assert annotate(raw, section('[On Play] Draw 1. [On Play] Gain 1 memory.')) == (raw, 0, 1)


def test_backfill_preserves_crlf_line_endings():
    raw = spec('  - when: on_play\n    process: []\n').replace('\n', '\r\n')
    updated, count, _ = annotate(raw, section('[On Play] Draw 1.'))
    assert count == 1
    assert updated == raw.replace('  - when:', '  - text: "[On Play] Draw 1."\r\n    when:')


def test_backfill_scope_and_keyword_matching():
    raw = spec('  - scope: inherited\n    kind: grant_keyword\n    keyword: Blocker\n')
    assert annotate(raw, section('＜Blocker＞', 'Inherited Effect'))[1] == 1
    assert annotate(raw, section('＜Blocker＞'))[1] == 0
    assert annotate(raw, section('＜Rush＞', 'Inherited Effect'))[1] == 0


def test_backfill_empty_unknown_and_inline_mapping_are_unchanged():
    raw = spec('  - {when: on_play}\n')
    assert annotate(raw, section('[On Play] Draw 1.')) == (raw, 0, 1)
    assert annotate(raw, []) == (raw, 0, 1)
    assert printed_spans(section('[Digivolve] Cost 1', 'Special Digivolution Condition')) == []


def test_backfill_keeps_embedded_keywords_with_their_effect():
    spans = printed_spans(section('[On Play] Draw 1. Then, ＜Recovery +1 (Deck)＞. [On Deletion] Draw 1.'))
    assert [s.text for s in spans] == [
        '[On Play] Draw 1. Then, ＜Recovery +1 (Deck)＞.', '[On Deletion] Draw 1.'
    ]


def test_backfill_invalid_yaml_is_an_error():
    with pytest.raises(Exception):
        annotate('effects: [', [])


def test_backfill_repeated_compound_markers_keep_their_own_bodies():
    spans = printed_spans(section('[On Play] [When Digivolving] Draw 1. [When Digivolving] [When Attacking] [Once Per Turn] Gain 1 memory.'))
    assert [s.text for s in spans] == [
        '[On Play] [When Digivolving] Draw 1.',
        '[When Digivolving] [When Attacking] [Once Per Turn] Gain 1 memory.',
    ]


def test_backfill_security_tag_in_effect_box_and_rule_suffix():
    raw = spec('  - when: on_security\n')
    updated, count, _ = annotate(raw, section('[Security] Play this card. (Rule) Name: Also treated as [X].'))
    assert count == 1
    assert 'text: "[Security] Play this card."' in updated
