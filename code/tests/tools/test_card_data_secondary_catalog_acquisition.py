"""Complete public catalog discovery must never certify a truncated search."""
import json
from urllib.parse import parse_qs, urlparse
import urllib.error

import pytest

from tools.card_data.cache import ContentAddressedResponseCache, RetryPolicy
from tools.card_data.errors import CardDataError
from tools.card_data.serialization import sha256_hex
from tools.card_data.sources.digimoncard_io import DigimonCardIOAdapter
from tools.card_data.live_reconcile import _secondary_snapshot


def row(cid, **extra):
    return dict(id=cid, name=cid, type="Option", color="Red", play_cost=1,
                main_effect="Draw 1.", series="Digimon Card Game", **extra)


class Response:
    status = 200
    def __init__(self, body): self.body = body
    def read(self): return self.body
    def close(self): pass


def adapter(tmp_path, ids, handler=None):
    calls, sleeps, bodies = [], [], {}
    def opener(request, **kwargs):
        url = request.full_url
        calls.append(url)
        query = parse_qs(urlparse(url).query)
        if urlparse(url).path.endswith("getAllCards"):
            value = ids
        else:
            assert int(query["limit"][0]) <= 1000
            requested = query["card"][0].split(",")
            value = handler(requested) if handler else [row(cid) for cid in requested]
        body = json.dumps(value).encode()
        bodies[url] = body
        return Response(body)
    cache = ContentAddressedResponseCache(tmp_path / "cache")
    return DigimonCardIOAdapter(opener=opener, sleep=sleeps.append, cache=cache,
                               retry_policy=RetryPolicy(max_attempts=1)), calls, sleeps, bodies, cache


def discover(*ids): return [dict(cardnumber=cid, name=cid) for cid in ids]


def test_complete_multibatch_truthful_urls_hashes_cache_and_manifest(tmp_path):
    ids = [f"BT1-{i:03}" for i in range(1, 102)]
    a, calls, sleeps, bodies, cache = adapter(tmp_path, discover(*ids))
    catalog = a.acquire_catalog()
    assert set(catalog.card_ids) == set(ids)
    assert len(catalog.acquisitions) == 2 and len(calls) == 3
    assert len(sleeps) == len(calls) and all(x >= .75 for x in sleeps)
    for url, body in bodies.items(): assert cache.get(sha256_hex(body)) == body
    for batch in catalog.acquisitions:
        assert batch.parsed.raw_payload == bodies[batch.parsed.source_url]
        assert all(o.source_url == batch.parsed.source_url and
                   o.raw_payload_digest == sha256_hex(bodies[o.source_url]) for o in batch.observations)
    snapshot = _secondary_snapshot(catalog.acquisitions, acquired_at="2026-10-29T00:00:00Z",
                                   discovery_url=catalog.discovery_url)
    assert snapshot.manifest.complete
    assert set(snapshot.manifest.source_locations) == set(calls)
    assert snapshot.manifest.expected_count == len(ids)


@pytest.mark.parametrize("ids", [[], {}, [dict(name="missing")], discover("BT1-001", "BT1-001")])
def test_invalid_discovery_refuses_before_record_requests(tmp_path, ids):
    a, calls, *_ = adapter(tmp_path, ids)
    with pytest.raises(CardDataError): a.acquire_catalog()
    assert len(calls) == 1


@pytest.mark.parametrize("mode", ["missing", "foreign", "late_failure"])
def test_incomplete_or_foreign_late_batch_never_returns_complete_catalog(tmp_path, mode):
    ids = [f"BT1-{i:03}" for i in range(1, 102)]
    def handler(requested):
        if requested[0] == ids[-1]:
            if mode == "late_failure":
                raise urllib.error.HTTPError("https://digimoncard.io/api-public/search", 422, "invalid", {}, None)
            return [] if mode == "missing" else [row("BT2-001")]
        return [row(cid) for cid in requested]
    a, *_ = adapter(tmp_path, discover(*ids), handler)
    with pytest.raises(CardDataError): a.acquire_catalog()


def test_saturated_batch_splits_without_using_parent_body_as_child_provenance(tmp_path):
    def handler(ids):
        return [row(ids[0], artist=str(i)) for i in range(1000)] if len(ids) > 1 else [row(ids[0])]
    a, calls, *_ = adapter(tmp_path, discover("BT1-001", "BT1-002"), handler)
    result = a.acquire_catalog()
    assert len(calls) == 4 and len(result.acquisitions) == 2
    assert all(len(b.parsed.card_ids) == 1 for b in result.acquisitions)


def test_saturated_single_identity_refuses_unprovable_completeness(tmp_path):
    a, *_ = adapter(tmp_path, discover("BT1-001"),
                    lambda ids: [row(ids[0], artist=str(i)) for i in range(1000)])
    with pytest.raises(CardDataError): a.acquire_catalog()


def test_duplicate_raw_rows_still_count_towards_saturation(tmp_path):
    a, *_ = adapter(tmp_path, discover("BT1-001"), lambda ids: [row(ids[0])] * 1000)
    with pytest.raises(CardDataError, match="saturated"):
        a.acquire_catalog()


def test_retry_attempts_are_also_throttled_and_cached(tmp_path):
    calls, sleeps = [], []
    body = json.dumps(discover("BT1-001")).encode()
    def opener(request, **kwargs):
        calls.append(request.full_url)
        if len(calls) == 1:
            raise urllib.error.HTTPError(request.full_url, 503, "busy", {}, None)
        return Response(body if "getAllCards" in request.full_url
                        else json.dumps([row("BT1-001")]).encode())
    a = DigimonCardIOAdapter(opener=opener, sleep=sleeps.append,
                            cache=ContentAddressedResponseCache(tmp_path / "cache"))
    assert a.acquire_catalog().card_ids == ("BT1-001",)
    assert len(calls) == 3
    # Three paced network attempts plus the retry policy's own backoff.
    assert sum(delay == .75 for delay in sleeps) == len(calls)


def test_existing_matching_observation_identity_is_reused(tmp_path):
    a, *_ = adapter(tmp_path, discover("BT1-001"))
    first = a.acquire_catalog().acquisitions[0].observations[0]
    a.existing_observations = {first.observation_id: first}
    assert a.acquire_catalog().acquisitions[0].observations[0] is first
