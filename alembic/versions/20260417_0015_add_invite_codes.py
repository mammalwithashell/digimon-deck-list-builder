"""add invite_codes table for alpha registration gating

Revision ID: 20260417_0015
Revises: 20260415_0014
Create Date: 2026-04-17 12:00:00
"""

from alembic import op
import sqlalchemy as sa


revision = "20260417_0015"
down_revision = "20260415_0014"
branch_labels = None
depends_on = None


def _has_table(name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return name in inspector.get_table_names()


def upgrade() -> None:
    if _has_table("invite_codes"):
        return
    op.create_table(
        "invite_codes",
        sa.Column("code", sa.String(), primary_key=True),
        sa.Column(
            "created_by_user_id",
            sa.String(),
            sa.ForeignKey("users.id", ondelete="SET NULL"),
            nullable=True,
        ),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column(
            "redeemed_by_user_id",
            sa.String(),
            sa.ForeignKey("users.id", ondelete="SET NULL"),
            nullable=True,
        ),
        sa.Column("redeemed_at", sa.DateTime(timezone=True), nullable=True),
        sa.Column("note", sa.Text(), nullable=True),
    )
    op.create_index(
        "idx_invite_codes_redeemed_by",
        "invite_codes",
        ["redeemed_by_user_id"],
    )


def downgrade() -> None:
    if not _has_table("invite_codes"):
        return
    op.drop_index("idx_invite_codes_redeemed_by", table_name="invite_codes")
    op.drop_table("invite_codes")
