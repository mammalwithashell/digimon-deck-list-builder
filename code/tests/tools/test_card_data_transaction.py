"""Writer-lock and transaction primitive tests for the card-data pipeline."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from tools.card_data.errors import WriteError
from tools.card_data.locking import (
    LockOwner,
    RepositoryWriterLock,
    StaleWriterLock,
    WriterLockUnavailable,
)
from tools.card_data.serialization import canonical_json_bytes, sha256_hex
from tools.card_data.transaction import (
    EMPTY_REVISION,
    ArtifactMutation,
    ConcurrentRevisionError,
    PlanToken,
    RecoveryRequiredError,
    RevisionMarker,
    StalePlanError,
    TransactionJournal,
    TransactionState,
    load_transaction_journal,
    prepare_transaction,
    read_consistent_revision,
    write_revision_marker,
    write_transaction_journal,
)


DIGEST_A = "a" * 64
DIGEST_B = "b" * 64
DIGEST_C = "c" * 64
DIGEST_D = "d" * 64


def _plan(**overrides) -> PlanToken:
    values = {
        "requested_scope": "release:EX12",
        "base_revision": EMPTY_REVISION,
        "staged_digest": DIGEST_B,
        "diff_digest": DIGEST_C,
    }
    values.update(overrides)
    return PlanToken.create(**values)


def _mutation(path: str = "data/cards.json", **overrides) -> ArtifactMutation:
    values = {
        "artifact_path": path,
        "staged_path": f"data/.card_data_staging/next/{Path(path).name}",
        "new_digest": DIGEST_B,
    }
    values.update(overrides)
    return ArtifactMutation(**values)


def _write_artifact(root: Path, relative_path: str, body: bytes) -> str:
    path = root / relative_path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(body)
    return sha256_hex(body)


def test_plan_token_binds_scope_base_staged_and_diff_digests():
    token = _plan(reviewed_adjudication_digests=(DIGEST_D,))

    assert PlanToken.from_dict(token.to_dict()) == token
    assert _plan(requested_scope="release:BT1").token_id != token.token_id
    assert _plan(base_revision=DIGEST_A).token_id != token.token_id
    assert _plan(staged_digest=DIGEST_A).token_id != token.token_id
    assert _plan(diff_digest=DIGEST_A).token_id != token.token_id

    corrupted = {**token.to_dict(), "diff_digest": DIGEST_A}
    with pytest.raises(ValueError, match="token_id"):
        PlanToken.from_dict(corrupted)


def test_repository_writer_lock_is_exclusive(tmp_path: Path):
    lock_path = tmp_path / "data/.writer.lock"
    with RepositoryWriterLock(tmp_path, lock_path=lock_path) as first:
        assert first.is_held
        assert first.owner is not None
        with pytest.raises(WriterLockUnavailable):
            RepositoryWriterLock(tmp_path, lock_path=lock_path).acquire()
        with pytest.raises(WriterLockUnavailable):
            RepositoryWriterLock(
                tmp_path, lock_path=lock_path, recover_stale=True
            ).acquire()

    with RepositoryWriterLock(tmp_path, lock_path=lock_path) as second:
        assert second.is_held
        assert second.owner is not None
        assert second.owner.recovered_lock_id is None


def test_stale_writer_metadata_requires_explicit_recovery(tmp_path: Path):
    lock_path = tmp_path / "data/.writer.lock"
    lock_path.parent.mkdir(parents=True)
    stale = LockOwner(
        schema_version=1,
        lock_id="dead-owner",
        repository_root=str(tmp_path.resolve()),
        pid=999_999,
        hostname="test-host",
        acquired_at=1.0,
    )
    lock_path.write_bytes(canonical_json_bytes(stale.to_dict()))

    with pytest.raises(StaleWriterLock):
        RepositoryWriterLock(tmp_path, lock_path=lock_path).acquire()

    with RepositoryWriterLock(
        tmp_path, lock_path=lock_path, recover_stale=True
    ) as recovered:
        assert recovered.owner is not None
        assert recovered.owner.recovered_lock_id == "dead-owner"


def test_transaction_journal_enforces_commit_and_reverse_rollback_order():
    journal = TransactionJournal.begin(
        _plan(),
        (
            _mutation("data/cards.json"),
            _mutation("data/card_official.json", new_digest=DIGEST_C),
        ),
    ).start_publishing()
    ordered_paths = [item.artifact_path for item in journal.mutations]

    with pytest.raises(ValueError, match="replace every artifact"):
        journal.commit()

    for path in ordered_paths:
        journal = journal.mark_replaced(path)
    committed = journal.commit()
    assert committed.state is TransactionState.COMMITTED

    rolling_back = journal.start_rollback("fault injected before marker commit")
    for path in reversed(ordered_paths):
        rolling_back = rolling_back.mark_rolled_back(path)
    rolled_back = rolling_back.finish_rollback()
    assert rolled_back.state is TransactionState.ROLLED_BACK
    assert set(rolled_back.rolled_back_paths) == set(ordered_paths)


def test_transaction_journal_accepts_only_exact_prefix_batches():
    journal = TransactionJournal.begin(
        _plan(),
        tuple(
            _mutation(f"data/cards-{index}.json") for index in range(5)
        ),
    ).start_publishing()
    ordered = tuple(item.artifact_path for item in journal.mutations)

    with pytest.raises(ValueError, match="must not be empty"):
        journal.mark_replaced_batch(())
    with pytest.raises(ValueError, match="next artifact replacement batch"):
        journal.mark_replaced_batch((ordered[1], ordered[0]))

    journal = journal.mark_replaced_batch(ordered[:3])
    journal = journal.mark_replaced_batch(ordered[3:])
    assert journal.replaced_paths == ordered

    rolling_back = journal.start_rollback("batched rollback")
    reverse_order = tuple(reversed(ordered))
    with pytest.raises(ValueError, match="next artifact rollback batch"):
        rolling_back.mark_rolled_back_batch(reverse_order[1:3])
    rolling_back = rolling_back.mark_rolled_back_batch(reverse_order[:2])
    rolling_back = rolling_back.mark_rolled_back_batch(reverse_order[2:])
    assert rolling_back.finish_rollback().rolled_back_paths == reverse_order


def test_committed_rollback_requires_explicit_transition_and_reason():
    journal = TransactionJournal.begin(_plan(), (_mutation(),)).start_publishing()
    journal = journal.mark_replaced("data/cards.json")
    committed = journal.commit()

    with pytest.raises(ValueError, match="committed transaction"):
        journal.start_committed_rollback("too early")
    with pytest.raises(ValueError, match="reason"):
        committed.start_committed_rollback("   ")

    rolling_back = committed.start_committed_rollback("operator approved")
    assert rolling_back.state is TransactionState.ROLLING_BACK
    assert rolling_back.rollback_reason == "operator approved"
    assert rolling_back.replaced_paths == ("data/cards.json",)


def test_mutation_requires_rollback_metadata_for_existing_artifact():
    with pytest.raises(ValueError, match="rollback_path"):
        _mutation(prior_digest=DIGEST_A)
    with pytest.raises(ValueError, match="newly created artifact"):
        _mutation(rollback_path="data/.card_data_rollback/old/cards.json")

    mutation = _mutation(
        prior_digest=DIGEST_A,
        rollback_path="data/.card_data_rollback/old/cards.json",
    )
    assert ArtifactMutation.from_dict(mutation.to_dict()) == mutation


def test_journal_identity_rejects_tampered_rollback_or_staging_metadata():
    journal = TransactionJournal.begin(_plan(), (_mutation(),))
    tampered = journal.to_dict()
    tampered["mutations"][0]["staged_path"] = "data/.card_data_staging/other.json"

    with pytest.raises(ValueError, match="transaction_id"):
        TransactionJournal.from_dict(tampered)


def test_prepare_transaction_performs_base_revision_cas_before_journal_write(
    tmp_path: Path,
):
    marker_path = tmp_path / "data/card_data_revision.json"
    journal_path = tmp_path / "data/.card_data_transaction.json"
    artifact_digest = _write_artifact(tmp_path, "data/cards.json", b"old")
    marker = RevisionMarker.create({"data/cards.json": artifact_digest})
    write_revision_marker(marker_path, marker)

    stale_plan = _plan(base_revision=DIGEST_A)
    with RepositoryWriterLock(tmp_path) as writer_lock:
        with pytest.raises(StalePlanError):
            prepare_transaction(
                stale_plan,
                (_mutation(),),
                marker_path=marker_path,
                journal_path=journal_path,
                writer_lock=writer_lock,
            )
        assert not journal_path.exists()

        current_plan = _plan(base_revision=marker.revision)
        prepared = prepare_transaction(
            current_plan,
            (_mutation(),),
            marker_path=marker_path,
            journal_path=journal_path,
            writer_lock=writer_lock,
        )

    assert prepared.state is TransactionState.PREPARED
    assert load_transaction_journal(journal_path) == prepared


def test_prepare_transaction_requires_held_writer_lock(tmp_path: Path):
    writer_lock = RepositoryWriterLock(tmp_path)
    with pytest.raises(WriteError, match="requires the writer lock"):
        prepare_transaction(
            _plan(),
            (_mutation(),),
            marker_path=tmp_path / "data/card_data_revision.json",
            journal_path=tmp_path / "data/.card_data_transaction.json",
            writer_lock=writer_lock,
        )


def test_transaction_metadata_must_use_the_locked_repository(tmp_path: Path):
    other = tmp_path.parent / f"{tmp_path.name}-outside"
    with RepositoryWriterLock(tmp_path) as writer_lock:
        with pytest.raises(WriteError, match="locked repository"):
            prepare_transaction(
                _plan(),
                (_mutation(),),
                marker_path=other / "card_data_revision.json",
                journal_path=other / ".card_data_transaction.json",
                writer_lock=writer_lock,
            )


def test_consistent_reader_returns_verified_file_values(tmp_path: Path):
    marker_path = tmp_path / "data/card_data_revision.json"
    journal_path = tmp_path / "data/.card_data_transaction.json"
    digest = _write_artifact(tmp_path, "data/cards.json", b'{"ok":true}')
    marker = RevisionMarker.create({"data/cards.json": digest})
    write_revision_marker(marker_path, marker)

    result = read_consistent_revision(
        tmp_path,
        marker_path=marker_path,
        journal_path=journal_path,
        loaders={"data/cards.json": lambda raw: json.loads(raw)},
    )

    assert result.revision == marker.revision
    assert result.values["data/cards.json"] == {"ok": True}


def test_consistent_reader_rejects_incomplete_journal(tmp_path: Path):
    marker_path = tmp_path / "data/card_data_revision.json"
    journal_path = tmp_path / "data/.card_data_transaction.json"
    digest = _write_artifact(tmp_path, "data/cards.json", b"old")
    marker = RevisionMarker.create({"data/cards.json": digest})
    write_revision_marker(marker_path, marker)
    journal = TransactionJournal.begin(
        _plan(base_revision=marker.revision), (_mutation(),)
    )
    write_transaction_journal(journal_path, journal)

    with pytest.raises(RecoveryRequiredError, match="incomplete"):
        read_consistent_revision(
            tmp_path,
            marker_path=marker_path,
            journal_path=journal_path,
        )


def test_consistent_reader_rejects_hash_mismatch(tmp_path: Path):
    marker_path = tmp_path / "data/card_data_revision.json"
    journal_path = tmp_path / "data/.card_data_transaction.json"
    digest = _write_artifact(tmp_path, "data/cards.json", b"old")
    marker = RevisionMarker.create({"data/cards.json": digest})
    write_revision_marker(marker_path, marker)
    (tmp_path / "data/cards.json").write_bytes(b"hand edited")

    with pytest.raises(RecoveryRequiredError, match="digest"):
        read_consistent_revision(
            tmp_path,
            marker_path=marker_path,
            journal_path=journal_path,
        )


def test_reader_detects_revision_change_after_artifact_load(tmp_path: Path):
    marker_path = tmp_path / "data/card_data_revision.json"
    journal_path = tmp_path / "data/.card_data_transaction.json"
    old_digest = _write_artifact(tmp_path, "data/cards.json", b"old")
    new_digest = _write_artifact(tmp_path, "data/card_official.json", b"new")
    old_marker = RevisionMarker.create({"data/cards.json": old_digest})
    new_marker = RevisionMarker.create(
        {
            "data/cards.json": old_digest,
            "data/card_official.json": new_digest,
        }
    )
    write_revision_marker(marker_path, old_marker)

    def commit_during_read(raw: bytes) -> bytes:
        write_revision_marker(marker_path, new_marker)
        return raw

    with pytest.raises(ConcurrentRevisionError):
        read_consistent_revision(
            tmp_path,
            marker_path=marker_path,
            journal_path=journal_path,
            loaders={"data/cards.json": commit_during_read},
            max_attempts=1,
        )

    result = read_consistent_revision(
        tmp_path,
        marker_path=marker_path,
        journal_path=journal_path,
    )
    assert result.revision == new_marker.revision
