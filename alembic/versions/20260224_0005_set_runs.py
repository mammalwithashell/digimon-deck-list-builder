"""add ai set run tables and ai_tasks set_run_id

Revision ID: 20260224_0005
Revises: 20260223_0004
Create Date: 2026-02-24 14:00:00
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260224_0005"
down_revision = "20260223_0004"
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


def upgrade() -> None:
    if not _has_table("ai_set_runs"):
        op.create_table(
            "ai_set_runs",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("set_id", sa.String(), nullable=False),
            sa.Column("status", sa.String(), nullable=False, server_default="running"),
            sa.Column("stage", sa.String(), nullable=False, server_default="qa"),
            sa.Column("run_mode", sa.String(), nullable=False, server_default="pr"),
            sa.Column("scope_profile", sa.String(), nullable=False, server_default="script"),
            sa.Column("model_name", sa.String(), nullable=True),
            sa.Column("created_by", sa.String(), nullable=True),
            sa.Column("max_total_cost_usd", sa.Float(), nullable=False, server_default="5"),
            sa.Column("failure_rate_stop", sa.Float(), nullable=False, server_default="0.3"),
            sa.Column("max_fix_tasks", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("qa_total", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("qa_completed", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("qa_failed", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("review_total", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("review_completed", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("review_failed", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("fix_total", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("fix_completed", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("fix_failed", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("fix_applied", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("stopped_reason", sa.Text(), nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("completed_at", sa.DateTime(timezone=True), nullable=True),
            sa.CheckConstraint("run_mode IN ('pr', 'main')", name="ck_ai_set_runs_run_mode"),
            sa.CheckConstraint(
                "scope_profile IN ('script', 'script_engine', 'script_engine_transpiler')",
                name="ck_ai_set_runs_scope_profile",
            ),
            sa.CheckConstraint(
                "status IN ('running', 'completed', 'failed', 'canceled')",
                name="ck_ai_set_runs_status",
            ),
            sa.CheckConstraint(
                "stage IN ('qa', 'review', 'fix', 'completed', 'canceled')",
                name="ck_ai_set_runs_stage",
            ),
            sa.ForeignKeyConstraint(["created_by"], ["users.id"], ondelete="SET NULL"),
            sa.PrimaryKeyConstraint("id"),
        )
        op.create_index("idx_ai_set_runs_set_id", "ai_set_runs", ["set_id"])
        op.create_index("idx_ai_set_runs_status", "ai_set_runs", ["status"])
        op.create_index("idx_ai_set_runs_created_at", "ai_set_runs", ["created_at"])

    if not _has_table("ai_set_run_items"):
        op.create_table(
            "ai_set_run_items",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("set_run_id", sa.String(), nullable=False),
            sa.Column("card_id", sa.String(), nullable=False),
            sa.Column("set_id", sa.String(), nullable=False),
            sa.Column("module_name", sa.String(), nullable=False),
            sa.Column("review_faithful", sa.Integer(), nullable=True),
            sa.Column("issue_id", sa.String(), nullable=True),
            sa.Column("qa_task_id", sa.String(), nullable=True),
            sa.Column("review_task_id", sa.String(), nullable=True),
            sa.Column("fix_task_id", sa.String(), nullable=True),
            sa.Column("fix_apply_status", sa.String(), nullable=False, server_default="pending"),
            sa.Column("commit_sha", sa.String(), nullable=True),
            sa.Column("pr_url", sa.String(), nullable=True),
            sa.Column("error_text", sa.Text(), nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.CheckConstraint(
                "fix_apply_status IN ('pending', 'applied', 'failed', 'skipped')",
                name="ck_ai_set_run_items_fix_apply_status",
            ),
            sa.ForeignKeyConstraint(["set_run_id"], ["ai_set_runs.id"], ondelete="CASCADE"),
            sa.ForeignKeyConstraint(["issue_id"], ["issues.id"], ondelete="SET NULL"),
            sa.ForeignKeyConstraint(["qa_task_id"], ["ai_tasks.id"], ondelete="SET NULL"),
            sa.ForeignKeyConstraint(["review_task_id"], ["ai_tasks.id"], ondelete="SET NULL"),
            sa.ForeignKeyConstraint(["fix_task_id"], ["ai_tasks.id"], ondelete="SET NULL"),
            sa.PrimaryKeyConstraint("id"),
        )
        op.create_index("idx_ai_set_run_items_set_run_id", "ai_set_run_items", ["set_run_id"])
        op.create_index("idx_ai_set_run_items_card_id", "ai_set_run_items", ["card_id"])
        op.create_index(
            "idx_ai_set_run_items_fix_apply_status",
            "ai_set_run_items",
            ["fix_apply_status"],
        )

    if _has_table("ai_tasks") and not _has_column("ai_tasks", "set_run_id"):
        with op.batch_alter_table("ai_tasks") as batch_op:
            batch_op.add_column(sa.Column("set_run_id", sa.String(), nullable=True))
            batch_op.create_foreign_key(
                "fk_ai_tasks_set_run_id",
                "ai_set_runs",
                ["set_run_id"],
                ["id"],
                ondelete="SET NULL",
            )

    if _has_table("ai_tasks") and not _has_index("ai_tasks", "idx_ai_tasks_set_run_id"):
        op.create_index("idx_ai_tasks_set_run_id", "ai_tasks", ["set_run_id"])


def downgrade() -> None:
    if _has_table("ai_tasks") and _has_index("ai_tasks", "idx_ai_tasks_set_run_id"):
        op.drop_index("idx_ai_tasks_set_run_id", table_name="ai_tasks")

    if _has_table("ai_tasks") and _has_column("ai_tasks", "set_run_id"):
        with op.batch_alter_table("ai_tasks") as batch_op:
            try:
                batch_op.drop_constraint("fk_ai_tasks_set_run_id", type_="foreignkey")
            except Exception:
                pass
            batch_op.drop_column("set_run_id")

    if _has_table("ai_set_run_items"):
        op.drop_index("idx_ai_set_run_items_fix_apply_status", table_name="ai_set_run_items")
        op.drop_index("idx_ai_set_run_items_card_id", table_name="ai_set_run_items")
        op.drop_index("idx_ai_set_run_items_set_run_id", table_name="ai_set_run_items")
        op.drop_table("ai_set_run_items")

    if _has_table("ai_set_runs"):
        op.drop_index("idx_ai_set_runs_created_at", table_name="ai_set_runs")
        op.drop_index("idx_ai_set_runs_status", table_name="ai_set_runs")
        op.drop_index("idx_ai_set_runs_set_id", table_name="ai_set_runs")
        op.drop_table("ai_set_runs")
