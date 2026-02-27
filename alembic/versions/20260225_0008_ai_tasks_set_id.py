"""add set_id to ai_tasks and backfill

Revision ID: 20260225_0008
Revises: 20260225_0007
Create Date: 2026-02-25 15:00:00
"""

from __future__ import annotations

import json

from alembic import op
import sqlalchemy as sa


revision = "20260225_0008"
down_revision = "20260225_0007"
branch_labels = None
depends_on = None


def _has_table(name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return name in inspector.get_table_names()


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = [col["name"] for col in inspector.get_columns(table_name)]
    return column_name in columns


def _has_index(table_name: str, index_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return index_name in {idx["name"] for idx in inspector.get_indexes(table_name)}


def _set_id_from_card_id(card_id: object) -> str | None:
    if not isinstance(card_id, str):
        return None
    text = card_id.strip().upper()
    if not text:
        return None
    if "-" in text:
        return text.split("-", 1)[0].lower()
    if "_" in text:
        return text.split("_", 1)[0].lower()
    return None


def _extract_set_id(payload_json: object) -> str | None:
    if not isinstance(payload_json, str) or not payload_json.strip():
        return None
    try:
        payload = json.loads(payload_json)
    except json.JSONDecodeError:
        return None
    if not isinstance(payload, dict):
        return None

    set_value = payload.get("set_id")
    if isinstance(set_value, str) and set_value.strip():
        return set_value.strip().lower()

    card_id = payload.get("card_id")
    card_set = _set_id_from_card_id(card_id)
    if card_set:
        return card_set

    cards = payload.get("cards")
    if isinstance(cards, list):
        explicit_set_ids: set[str] = set()
        card_set_ids: set[str] = set()
        for entry in cards:
            if not isinstance(entry, dict):
                continue
            raw_set_id = entry.get("set_id")
            if isinstance(raw_set_id, str) and raw_set_id.strip():
                explicit_set_ids.add(raw_set_id.strip().lower())
            parsed_set_id = _set_id_from_card_id(entry.get("card_id"))
            if parsed_set_id:
                card_set_ids.add(parsed_set_id)
        if len(explicit_set_ids) == 1:
            return next(iter(explicit_set_ids))
        if len(explicit_set_ids) == 0 and len(card_set_ids) == 1:
            return next(iter(card_set_ids))
    return None


def upgrade() -> None:
    if not _has_table("ai_tasks"):
        return

    if not _has_column("ai_tasks", "set_id"):
        with op.batch_alter_table("ai_tasks") as batch_op:
            batch_op.add_column(sa.Column("set_id", sa.String(), nullable=True))

    if _has_column("ai_tasks", "set_id") and not _has_index("ai_tasks", "idx_ai_tasks_set_id"):
        op.create_index("idx_ai_tasks_set_id", "ai_tasks", ["set_id"])

    bind = op.get_bind()

    if _has_table("ai_fix_batches"):
        bind.execute(
            sa.text(
                """
                UPDATE ai_tasks
                SET set_id = (
                    SELECT lower(ai_fix_batches.set_id)
                    FROM ai_fix_batches
                    WHERE ai_fix_batches.id = ai_tasks.batch_id
                )
                WHERE ai_tasks.set_id IS NULL
                  AND ai_tasks.batch_id IS NOT NULL
                """
            )
        )

    if _has_table("ai_set_runs"):
        bind.execute(
            sa.text(
                """
                UPDATE ai_tasks
                SET set_id = (
                    SELECT lower(ai_set_runs.set_id)
                    FROM ai_set_runs
                    WHERE ai_set_runs.id = ai_tasks.set_run_id
                )
                WHERE ai_tasks.set_id IS NULL
                  AND ai_tasks.set_run_id IS NOT NULL
                """
            )
        )

    rows = bind.execute(
        sa.text("SELECT id, payload_json FROM ai_tasks WHERE set_id IS NULL")
    ).fetchall()
    for row in rows:
        set_id = _extract_set_id(row.payload_json)
        if set_id:
            bind.execute(
                sa.text("UPDATE ai_tasks SET set_id = :set_id WHERE id = :task_id"),
                {"set_id": set_id, "task_id": row.id},
            )


def downgrade() -> None:
    if not _has_table("ai_tasks"):
        return

    if _has_index("ai_tasks", "idx_ai_tasks_set_id"):
        op.drop_index("idx_ai_tasks_set_id", table_name="ai_tasks")

    if _has_column("ai_tasks", "set_id"):
        with op.batch_alter_table("ai_tasks") as batch_op:
            batch_op.drop_column("set_id")
