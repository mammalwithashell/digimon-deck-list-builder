"""add ai_models table for ONNX model management

Revision ID: 20260417_0014
Revises: 27fb7ae4b4a4
Create Date: 2026-04-17
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260417_0014"
down_revision = "27fb7ae4b4a4"
branch_labels = None
depends_on = None


def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()


def upgrade() -> None:
    if not _has_table("ai_models"):
        op.create_table(
            "ai_models",
            sa.Column("id", sa.String(), primary_key=True),
            sa.Column("name", sa.String(), nullable=False),
            sa.Column("model_type", sa.String(), nullable=False),
            sa.Column("tensor_size", sa.Integer(), nullable=True),
            sa.Column("action_space_size", sa.Integer(), nullable=True),
            sa.Column("engine_commit", sa.String(), nullable=True),
            sa.Column("trained_at", sa.DateTime(timezone=True), nullable=True),
            sa.Column("file_sha256", sa.String(), nullable=True),
            sa.Column("file_size_bytes", sa.Integer(), nullable=True),
            sa.Column("spaces_key", sa.String(), nullable=False),
            sa.Column(
                "deck_id",
                sa.String(),
                sa.ForeignKey("decks.id", ondelete="SET NULL"),
                nullable=True,
            ),
            sa.Column(
                "uploaded_by",
                sa.String(),
                sa.ForeignKey("users.id", ondelete="SET NULL"),
                nullable=True,
            ),
            sa.Column("published", sa.Boolean(), nullable=False, server_default="0"),
            sa.Column("state", sa.String(), nullable=False, server_default="pending"),
            sa.Column("notes", sa.Text(), nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.CheckConstraint(
                "model_type IN ('mlp', 'lstm')",
                name="ck_ai_models_model_type",
            ),
            sa.CheckConstraint(
                "state IN ('pending', 'uploaded', 'failed')",
                name="ck_ai_models_state",
            ),
            sa.UniqueConstraint("spaces_key", name="uq_ai_models_spaces_key"),
        )
        op.create_index("idx_ai_models_deck_id", "ai_models", ["deck_id"])
        op.create_index("idx_ai_models_published", "ai_models", ["published"])


def downgrade() -> None:
    if _has_table("ai_models"):
        op.drop_index("idx_ai_models_published", table_name="ai_models")
        op.drop_index("idx_ai_models_deck_id", table_name="ai_models")
        op.drop_table("ai_models")
