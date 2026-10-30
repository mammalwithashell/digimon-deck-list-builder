"""Tests for the DigiLab public-API ingestion path in tools/meta_loader.py.

Every HTTP exchange is served from the recorded, hand-trimmed fixtures under
``fixtures/digilab_api/`` through a fake ``requests``-shaped session. No test
here opens a socket, and no test needs a real DIGILAB_API_KEY.
"""

from __future__ import annotations

import json
import logging
import os
import sys
import urllib.parse
from typing import Any, Dict, List, Optional

import pytest

_CODE_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
if _CODE_ROOT not in sys.path:
    sys.path.insert(0, _CODE_ROOT)

from tools import meta_loader
from tools.meta_loader import (
    DIGILAB_API_BASE,
    DIGILAB_API_KEY_ENV,
    DIGILAB_ATTRIBUTION,
    DeckIngestor,
    DigiLabApiClient,
    DigiLabApiKeyMissing,
    IngestedDeck,
    SOURCE_PRIORITY,
    _TokenBucket,
)

FIXTURES = os.path.join(os.path.dirname(os.path.abspath(__file__)), "fixtures", "digilab_api")
FAKE_KEY = "dl_k_TESTSECRET_do_not_print"


def _fixture(name: str) -> Dict[str, Any]:
    with open(os.path.join(FIXTURES, name), "r", encoding="utf-8") as f:
        return json.load(f)


# ─── Fake HTTP layer ────────────────────────────────────────────────


class FakeResponse:
    def __init__(self, status_code: int, payload: Any = None, headers: Optional[Dict[str, str]] = None):
        self.status_code = status_code
        self._payload = payload if payload is not None else {}
        self.headers = headers or {}
        self.text = json.dumps(self._payload)

    def json(self) -> Any:
        return self._payload


class FakeSession:
    """Routes ``GET <base>/<path>?params`` to fixture payloads and records calls."""

    def __init__(self) -> None:
        self.calls: List[Dict[str, Any]] = []
        # Optional per-path queue of canned responses (used for 429 tests).
        self.queued: Dict[str, List[FakeResponse]] = {}

    def get(self, url: str, params: Optional[Dict[str, Any]] = None, headers: Optional[Dict[str, str]] = None, timeout: Any = None) -> FakeResponse:
        params = dict(params or {})
        parsed = urllib.parse.urlparse(url)
        path = parsed.path
        self.calls.append({"url": url, "path": path, "params": params, "headers": dict(headers or {})})

        if path in self.queued and self.queued[path]:
            return self.queued[path].pop(0)

        page = int(params.get("page", 1))
        if path.endswith("/api/tournaments"):
            return FakeResponse(200, _fixture("tournaments_p1.json") if page == 1 else {"data": [], "pagination": {"page": page, "per_page": 100, "total": 2, "total_pages": 1}})
        if path.endswith("/api/decklists"):
            return FakeResponse(200, _fixture("decklists_p1.json") if page == 1 else {"data": [], "pagination": {"page": page, "per_page": 100, "total": 3, "total_pages": 1}})
        m = path.rsplit("/api/decklist/", 1)
        if len(m) == 2 and m[1].isdigit():
            name = f"decklist_{m[1]}.json"
            if os.path.exists(os.path.join(FIXTURES, name)):
                return FakeResponse(200, _fixture(name))
            return FakeResponse(404, {"error": "Resource not found"})
        return FakeResponse(404, {"error": "Resource not found"})

    def decklist_ids_fetched(self) -> List[int]:
        return [int(c["path"].rsplit("/", 1)[1]) for c in self.calls if "/api/decklist/" in c["path"]]


@pytest.fixture
def api_key(monkeypatch):
    monkeypatch.setenv(DIGILAB_API_KEY_ENV, FAKE_KEY)
    return FAKE_KEY


@pytest.fixture
def session() -> FakeSession:
    return FakeSession()


def _client(session: FakeSession, **kw) -> DigiLabApiClient:
    """A client with instant sleeps and no pacing, for deterministic tests."""
    kw.setdefault("sleep", lambda s: None)
    kw.setdefault("rate_per_min", 100000)
    return DigiLabApiClient(session=session, **kw)


# ─── Key handling ───────────────────────────────────────────────────


class TestApiKey:
    def test_missing_key_fails_fast_without_network(self, monkeypatch, session):
        monkeypatch.delenv(DIGILAB_API_KEY_ENV, raising=False)
        ingestor = DeckIngestor()
        with pytest.raises(DigiLabApiKeyMissing) as exc:
            ingestor.scrape_digilab_api(formats={"EX13"}, client=_client(session))
        assert DIGILAB_API_KEY_ENV in str(exc.value)
        assert session.calls == []

    def test_key_sent_in_header_and_never_logged(self, api_key, session, caplog):
        caplog.set_level(logging.DEBUG)
        ingestor = DeckIngestor()
        ingestor.scrape_digilab_api(formats={"EX13"}, client=_client(session))
        assert session.calls, "expected at least one API call"
        for call in session.calls:
            assert call["headers"].get("X-API-Key") == FAKE_KEY
        assert FAKE_KEY not in caplog.text
        for call in session.calls:
            assert FAKE_KEY not in call["url"]
            assert FAKE_KEY not in json.dumps(call["params"])

    def test_rejected_key_error_does_not_echo_key(self, api_key, session):
        session.queued["/api/tournaments"] = [FakeResponse(403, {"error": "Forbidden"})]
        ingestor = DeckIngestor()
        with pytest.raises(RuntimeError) as exc:
            ingestor.scrape_digilab_api(formats={"EX13"}, client=_client(session))
        assert FAKE_KEY not in str(exc.value)
        assert "403" in str(exc.value)


# ─── Walk + record shape ────────────────────────────────────────────


class TestWalk:
    def test_walk_produces_deck_records(self, api_key, session):
        ingestor = DeckIngestor()
        n = ingestor.scrape_digilab_api(formats={"EX13"}, client=_client(session))

        # 9002 has decklist: null (unparsed) and must be skipped, not crash.
        assert n == 2
        assert sorted(session.decklist_ids_fetched()) == [9001, 9002, 9003]

        decks = [d for a in ingestor.archetypes.values() for d in a.decklists]
        assert len(decks) == 2
        by_id = {d.source_url: d for d in decks}
        alice = by_id[f"{DIGILAB_API_BASE}/decklist/9001"]
        assert alice.source == "digilab_api"
        assert alice.format_tag == "EX13"
        assert alice.placement == "1"
        assert alice.is_top_cut is True
        assert alice.event_date == "2026-09-14"
        assert alice.event_players == 18
        assert alice.player_name == "Alice"
        assert alice.archetype_name == "Blue Flare"
        assert alice.deck_id.startswith("digilab_api_")
        assert len(alice.card_ids) == 54
        assert alice.card_counts["BT10-029"] == 4
        assert alice.card_counts["BT10-002"] == 4  # eggs are part of the list

        carol = by_id[f"{DIGILAB_API_BASE}/decklist/9003"]
        assert carol.placement == "2"
        assert carol.event_players == 12
        # Alt-art suffix folded onto the base id; garbage row dropped.
        assert carol.card_counts["BT10-038"] == 2
        assert "not-a-card" not in carol.card_counts
        assert len(carol.card_ids) == 54

        arch = ingestor.archetypes["Blue Flare"]
        assert arch.primary_color == "Blue"
        assert arch.display_card_id == "BT10-029"

    def test_format_and_since_are_sent_as_query_params(self, api_key, session):
        ingestor = DeckIngestor()
        ingestor.scrape_digilab_api(formats={"EX13"}, since="2026-08-01", client=_client(session))
        listing_calls = [c for c in session.calls if c["path"].endswith(("/api/tournaments", "/api/decklists"))]
        assert listing_calls
        for call in listing_calls:
            assert call["params"].get("format") == "EX13"
            assert call["params"].get("date_from") == "2026-08-01"
            assert int(call["params"].get("per_page", 0)) <= 100

    def test_max_tournaments_bounds_the_walk(self, api_key, session):
        ingestor = DeckIngestor()
        n = ingestor.scrape_digilab_api(formats={"EX13"}, max_tournaments=1, client=_client(session))
        # Only tournament 6301 (newest) is kept → results 9001 and 9002.
        assert sorted(session.decklist_ids_fetched()) == [9001, 9002]
        assert n == 1

    def test_already_ingested_results_are_not_refetched(self, api_key, session):
        ingestor = DeckIngestor()
        ingestor._add_deck_to_archetype(
            "Blue Flare",
            IngestedDeck(
                deck_id="digilab_api_existing",
                source="digilab_api",
                source_url=f"{DIGILAB_API_BASE}/decklist/9001",
                card_ids=["BT10-029"],
                card_counts={"BT10-029": 1},
                format_tag="EX13",
            ),
        )
        n = ingestor.scrape_digilab_api(formats={"EX13"}, client=_client(session))
        assert 9001 not in session.decklist_ids_fetched()
        assert n == 1

    def test_unknown_format_warns_but_proceeds(self, api_key, session, caplog):
        caplog.set_level(logging.WARNING)
        ingestor = DeckIngestor()
        ingestor.scrape_digilab_api(formats={"EX13", "ZZ99"}, client=_client(session))
        assert "ZZ99" in caplog.text

    def test_multiple_formats_walk_each(self, api_key, session):
        ingestor = DeckIngestor()
        ingestor.scrape_digilab_api(formats={"EX13", "EX12"}, client=_client(session))
        fmts = {c["params"].get("format") for c in session.calls if c["path"].endswith("/api/tournaments")}
        assert fmts == {"EX13", "EX12"}

    def test_source_priority_registered(self):
        assert SOURCE_PRIORITY["digilab_api"] >= SOURCE_PRIORITY["digilab"]


# ─── Decklist parsing ───────────────────────────────────────────────


class TestDecklistParsing:
    def test_accepts_plural_and_singular_keys(self):
        plural = {"eggs": [{"card_id": "BT1-001", "count": 4}], "digimon": [{"card_id": "BT1-010", "count": 2}], "tamers": [], "options": []}
        singular = {"egg": [{"card_id": "BT1-001", "count": 4}], "digimon": [{"card_id": "BT1-010", "count": 2}], "tamer": [], "option": []}
        assert DeckIngestor._digilab_api_decklist_to_card_counts(plural) == {"BT1-001": 4, "BT1-010": 2}
        assert DeckIngestor._digilab_api_decklist_to_card_counts(singular) == {"BT1-001": 4, "BT1-010": 2}

    def test_invalid_rows_skipped_and_counts_merged(self):
        raw = {
            "digimon": [
                {"card_id": "bt10-029", "count": 2},
                {"card_id": "BT10-029_P2", "count": 2},
                {"card_id": "", "count": 4},
                {"card_id": "BT10-031", "count": 0},
                {"card_id": "BT10-031", "count": "3"},
                {"name": "no id", "count": 1},
            ]
        }
        assert DeckIngestor._digilab_api_decklist_to_card_counts(raw) == {"BT10-029": 4, "BT10-031": 3}

    def test_none_decklist_is_empty(self):
        assert DeckIngestor._digilab_api_decklist_to_card_counts(None) == {}


# ─── Rate limiting + backoff ────────────────────────────────────────


class TestRateLimiting:
    def test_token_bucket_paces_requests(self):
        now = [0.0]
        slept: List[float] = []

        def clock():
            return now[0]

        def sleep(s):
            slept.append(s)
            now[0] += s

        bucket = _TokenBucket(rate_per_min=60, burst=1, clock=clock, sleep=sleep)
        bucket.acquire()  # burst token
        bucket.acquire()  # must wait ~1s for the next token
        bucket.acquire()
        assert len(slept) == 2
        assert all(abs(s - 1.0) < 1e-6 for s in slept)

    def test_default_rate_is_under_the_per_ip_limit(self):
        client = DigiLabApiClient(api_key="x", session=FakeSession(), sleep=lambda s: None)
        assert client.rate_per_min <= 60

    def test_429_honours_retry_after_then_succeeds(self, api_key, session):
        session.queued["/api/tournaments"] = [
            FakeResponse(429, {"error": "Rate limit exceeded"}, headers={"Retry-After": "3"}),
        ]
        slept: List[float] = []
        client = _client(session, sleep=slept.append)
        ingestor = DeckIngestor()
        n = ingestor.scrape_digilab_api(formats={"EX13"}, client=client)
        assert n == 2
        assert any(abs(s - 3.0) < 1e-6 for s in slept)
        tournaments_calls = [c for c in session.calls if c["path"].endswith("/api/tournaments")]
        assert len(tournaments_calls) == 2  # one 429, one 200

    def test_429_without_header_backs_off_exponentially(self, session):
        session.queued["/api/tournaments"] = [
            FakeResponse(429, {"error": "slow down"}),
            FakeResponse(429, {"error": "slow down"}),
        ]
        slept: List[float] = []
        client = _client(session, api_key="x", sleep=slept.append)
        client.get("/tournaments", {"page": 1})
        assert len(slept) == 2
        assert slept[1] > slept[0]

    def test_429_gives_up_after_max_retries(self, session):
        session.queued["/api/tournaments"] = [FakeResponse(429, {"error": "slow down"})] * 10
        client = _client(session, api_key="x", max_retries=2)
        with pytest.raises(RuntimeError, match="429"):
            client.get("/tournaments", {"page": 1})

    def test_delay_is_applied_between_requests(self, session):
        slept: List[float] = []
        client = _client(session, api_key="x", delay=0.7, sleep=slept.append)
        client.get("/tournaments", {"page": 1})
        client.get("/tournaments", {"page": 2})
        assert slept.count(0.7) == 2


# ─── Build flow + attribution ───────────────────────────────────────


class TestBuildFlow:
    def test_build_populates_format_stats_and_attribution(self, api_key, session, tmp_path):
        ingestor = DeckIngestor()
        ingestor.scrape_digilab_api(formats={"EX13"}, client=_client(session))
        ingestor.resolve_archetypes()
        ingestor.deduplicate()
        ingestor.compute_meta_stats()
        out = tmp_path / "deck_library.json"
        ingestor.save(str(out))

        data = json.loads(out.read_text(encoding="utf-8"))
        assert data["sources_attribution"]["digilab_api"] == DIGILAB_ATTRIBUTION
        assert "digilab.cards" in DIGILAB_ATTRIBUTION
        bf = data["archetypes"]["Blue Flare"]
        assert bf["format_stats"]["EX13"]["times_played"] == 2
        assert bf["format_stats"]["EX13"]["meta_share"] == 1.0
        assert bf["format_stats"]["EX13"]["sources"]["digilab_api"]["count"] == 2
        entry = bf["decklists"][0]
        assert entry["source"] == "digilab_api"
        assert entry["format"] == "EX13"
        assert entry["event_players"] in (18, 12)

        # Round-trips through load() unchanged.
        again = DeckIngestor()
        again.load(str(out))
        assert len(again.archetypes["Blue Flare"].decklists) == 2

    def test_rerun_does_not_duplicate_after_reload(self, api_key, session, tmp_path):
        out = tmp_path / "deck_library.json"
        first = DeckIngestor()
        first.scrape_digilab_api(formats={"EX13"}, client=_client(session))
        first.resolve_archetypes(); first.deduplicate(); first.compute_meta_stats(); first.save(str(out))

        second = DeckIngestor()
        second.load(str(out))
        fetched_before = len(session.decklist_ids_fetched())
        second.scrape_digilab_api(formats={"EX13"}, client=_client(session))
        # Already-known results are skipped on the wire, and nothing doubles up.
        assert len(session.decklist_ids_fetched()) == fetched_before + 1  # only the null 9002 is retried
        second.deduplicate()
        assert len(second.archetypes["Blue Flare"].decklists) == 2


# ─── CLI wiring ─────────────────────────────────────────────────────


class TestCli:
    def test_flags_are_wired_to_scrape_digilab_api(self, monkeypatch, tmp_path, api_key):
        captured: Dict[str, Any] = {}

        def fake_scrape(self, formats=None, since=None, max_tournaments=None, delay=0.0, client=None):
            captured.update(formats=formats, since=since, max_tournaments=max_tournaments, delay=delay)
            return 0

        monkeypatch.setattr(DeckIngestor, "scrape_digilab_api", fake_scrape)
        out = tmp_path / "lib.json"
        monkeypatch.setattr(sys, "argv", [
            "meta_loader.py",
            "--scrape-digilab-api",
            "--digilab-api-format", "EX13, BT26",
            "--digilab-api-since", "2026-08-01",
            "--digilab-api-max-tournaments", "2",
            "--digilab-api-delay", "0.3",
            "--output", str(out),
        ])
        meta_loader.main()
        assert captured["formats"] == {"EX13", "BT26"}
        assert captured["since"] == "2026-08-01"
        assert captured["max_tournaments"] == 2
        assert captured["delay"] == 0.3
        # The build step ran after the scrape and wrote the library.
        assert out.exists()
        assert "sources_attribution" in json.loads(out.read_text(encoding="utf-8"))

    def test_missing_key_exits_cleanly_without_writing(self, monkeypatch, tmp_path, capsys):
        monkeypatch.delenv(DIGILAB_API_KEY_ENV, raising=False)
        out = tmp_path / "lib.json"
        monkeypatch.setattr(sys, "argv", [
            "meta_loader.py", "--scrape-digilab-api", "--digilab-api-format", "EX13",
            "--output", str(out),
        ])
        with pytest.raises(SystemExit) as exc:
            meta_loader.main()
        assert exc.value.code == 2
        assert DIGILAB_API_KEY_ENV in capsys.readouterr().err
        assert not out.exists()

    def test_help_mentions_historical_db_path(self):
        doc = DeckIngestor.scrape_digilab.__doc__ or ""
        assert "2026-03-12" in doc
        assert "scrape-digilab-api" in doc
