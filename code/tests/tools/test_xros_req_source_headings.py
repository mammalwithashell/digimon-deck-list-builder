"""Source headings must not hide existing resolve-deck alternative hints."""
import json
from pathlib import Path

import pytest

from tools.xros_req_parser import parse

FIXTURES = Path(__file__).parents[1] / 'fixtures/card_data/xros_source_headings'
HEADER = '[Special Digivolution Condition]'


@pytest.mark.parametrize('cid',['BT15-020','BT15-021','BT15-032','BT15-045'])
def test_real_official_source_header_preserves_existing_hint(cid):
    f=json.loads((FIXTURES/f'{cid}.json').read_text(encoding='utf-8'))
    diff=f['diff'];source=f['source_units'][0]
    assert f['observations'][source['observation_id']]['extracted']['text_sections'][source['section_index']]['text']==source['text']
    assert diff['old_value']==source['text']
    assert diff['new_value']==HEADER+' '+source['text']
    old=parse(diff['old_value']);new=parse(diff['new_value'])
    assert old.parsed and new==old


@pytest.mark.parametrize('body',[
    '[Digivolve] [Tsunomon]: Cost 0',
    '[Digivolve] Lv.5 w/[Greymon] in name: Cost 3',
    '[Digivolve] Lv.3 w/[Omnimon] in text: Cost 2',
    '[Digivolve] Lv.5 w/[Hero] trait: Cost 3',
    '[DNA Digivolve] [A] & [B]: Cost 0',
    '[App Fusion] [A] & [B] & [C]: Cost 0',
    '[Burst Digivolve] [A]: Cost 0',
])
def test_only_existing_digivolution_grammar_under_exact_heading(body):
    assert parse(HEADER+' '+body)==parse(body)


@pytest.mark.parametrize('raw',[
    HEADER,
    HEADER+' unknown condition',
    HEADER+' [Digivolve] [A] or [B]: Cost 0',
    HEADER+' [DNA Digivolution] [A] & [B]: Cost 0',
    HEADER+' DigiXros Requirements [Trait] [Name] x 2',
    HEADER+' Assembly -2: [A]',
    '[Special Play Condition] [Digivolve] [A]: Cost 0',
    '[Unknown Condition] [Digivolve] [A]: Cost 0',
    '[special digivolution condition] [Digivolve] [A]: Cost 0',
    HEADER+' '+HEADER+' [Digivolve] [A]: Cost 0',
    HEADER+' [Digivolve] [A]: Cost 0 trailing text',
    'Preamble '+HEADER+' [Digivolve] [A]: Cost 0',
])
def test_unknown_lines_retain_entire_original_heading(raw):
    result=parse(raw)
    assert not result.parsed and result.unparsed_lines==[raw]


def test_mixed_lines_keep_unknown_headers_and_repeated_hints():
    body='[Digivolve] [A]: Cost 0';unknown=HEADER+' unknown'
    result=parse(HEADER+' '+body+'\r\n'+unknown+'\n'+HEADER+' '+body)
    assert result.parsed==parse(body).parsed*2 and result.unparsed_lines==[unknown]


def test_standalone_heading_is_not_silently_removed_or_contextualized():
    body='[Digivolve] [A]: Cost 0';result=parse(HEADER+'\n'+body)
    assert result.parsed==parse(body).parsed and result.unparsed_lines==[HEADER]
