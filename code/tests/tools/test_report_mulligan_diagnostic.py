import pytest
from tools.cloud_search.report_mulligan_diagnostic import estimate


def test_source_block_members_stay_together_and_namespaces_are_separate():
    nodes=[dict(namespace='primary',block=0),dict(namespace='primary',block=0),
           dict(namespace='confirmation',block=0),dict(namespace='confirmation',block=0)]
    result=estimate(nodes,[0,1,0,1],repeats=100)
    assert result['source_blocks']==2
    assert result['roots']==4
    assert result['mean']==.5
    assert result['ci95']==[.5,.5]


def test_paired_constant_effect_is_preserved():
    nodes=[dict(namespace='primary',block=i) for i in range(12)]
    assert estimate(nodes,[-.25]*12,repeats=100)['ci95']==[-.25,-.25]


def test_missing_root_value_rejected():
    with pytest.raises(ValueError,match='estimate length'):
        estimate([dict(namespace='primary',block=0)],[])
