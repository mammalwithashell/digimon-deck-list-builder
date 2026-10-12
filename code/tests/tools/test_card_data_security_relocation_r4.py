"""No-header v2 envelope and printed-container refusals, actual immutable fixture."""
import hashlib
from pathlib import Path
import pytest
from test_card_data_security_relocation_proof import bundle, args, refresh, S
from tools.card_data import security_relocation_proof as proof

@pytest.mark.parametrize('change',['faq_container','missing_printed_info_column'])
def test_security_header_in_faq_or_unknown_container_is_not_printed_source(change):
    import re
    b=bundle();o=b['snapshot']['observations'][0];url=o['source_url'];raw=b['raw_pages'][url]
    if change=='faq_container':
        altered=re.sub(rb'(<dl class="cardInfoBoxSmall">\s*<dt class="cardInfoTitSmall">\[Security Effect\]</dt>.*?</dl>)',rb'<div class="cardFaqList">\1</div>',raw,flags=re.S)
    else:altered=raw.replace(b'class="cardInfoCol"',b'class="unrecognizedInfoCol"')
    assert altered!=raw;b['raw_pages'][url]=altered;refresh(b)
    assert not proof.prove(**args(b))['eligible']

@pytest.mark.parametrize('markup', ['<span hidden>[Security Effect]</span>', '<span style="display:none">[Security Effect]</span>', '<span>[Security Effect]</span>'])
def test_security_label_must_be_literal_text_without_child_elements(markup):
    b=bundle();o=b['snapshot']['observations'][0];url=o['source_url'];raw=b['raw_pages'][url]
    altered=raw.replace(b'[Security Effect]</dt>', markup.encode()+b'</dt>')
    assert altered!=raw;b['raw_pages'][url]=altered;refresh(b)
    assert not proof.prove(**args(b))['eligible']


def test_unwrapped_literal_security_label_remains_supported():
    assert proof.prove(**args(bundle()))['eligible']

@pytest.mark.parametrize('change',['foreign_selected','missing_selected','duplicate_provenance','boolean_ledger','unknown_ledger_field','unknown_correction_schema','boolean_correction_schema','unknown_correction_field','malformed_correction_row'])
def test_r1_complete_envelopes_and_selected_provenance(change):
    b=bundle();p=b['canonical']['cards']['AD1-019']['provenance'][S]
    if change=='foreign_selected':p['selected_observation_id']='f'*64
    elif change=='missing_selected':p.pop('selected_observation_id')
    elif change=='duplicate_provenance':p['observation_ids']*=2
    elif change=='boolean_ledger':b['ledger']['schema_version']=True
    elif change=='unknown_ledger_field':b['ledger']['future_rows']=[]
    elif change=='unknown_correction_schema':b['corrections']['schema_version']=999
    elif change=='boolean_correction_schema':b['corrections']['schema_version']=True
    elif change=='unknown_correction_field':b['corrections']['future_rows']=[]
    else:b['corrections']['corrections']=[{'card_id':'AD1-019'}]
    with pytest.raises(proof.ProofError):proof.prove(**args(b))


def test_r1_dependency_output_pins_complete_added_loader_closure():
    from tools.card_data import compatibility_corrections, compatibility_selections, corrections, errors
    output=proof.prove(**args(bundle()))['dependency_sha256']
    for module in (compatibility_corrections,compatibility_selections,corrections,errors):
        assert output[Path(module.__file__).name]==hashlib.sha256(Path(module.__file__).read_bytes()).hexdigest()
