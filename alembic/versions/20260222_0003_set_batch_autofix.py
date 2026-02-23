"""add ai set-batch autofix tables and task metadata

Revision ID: 20260222_0003
Revises: 20260222_0002
Create Date: 2026-02-22 16:20:00
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260222_0003"
down_revision = "20260222_0002"
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


def upgrade() -> None:
    if _has_table("ai_tasks"):
        with op.batch_alter_table("ai_tasks", recreate="always") as batch_op:
            batch_op.drop_constraint("ck_ai_tasks_task_type", type_="check")
            batch_op.create_check_constraint(
                "ck_ai_tasks_task_type",
                "task_type IN ('review_batch', 'qa_analysis', 'engine_audit', 'script_autofix')",
            )
            if not _has_column("ai_tasks", "batch_id"):
                batch_op.add_column(sa.Column("batch_id", sa.String(), nullable=True))
            if not _has_column("ai_tasks", "run_mode"):
                batch_op.add_column(sa.Column("run_mode", sa.String(), nullable=True))
            if not _has_column("ai_tasks", "scope_profile"):
                batch_op.add_column(sa.Column("scope_profile", sa.String(), nullable=True))
            batch_op.create_check_constraint(
                "ck_ai_tasks_run_mode",
                "run_mode IN ('pr', 'main') OR run_mode IS NULL",
            )
            batch_op.create_check_constraint(
                "ck_ai_tasks_scope_profile",
                "scope_profile IN ('script', 'script_engine', 'script_engine_transpiler') OR scope_profile IS NULL",
            )

    if _has_table("ai_tasks"):
        bind = op.get_bind()
        inspector = sa.inspect(bind)
        existing_indexes = {idx["name"] for idx in inspector.get_indexes("ai_tasks")}
        if "idx_ai_tasks_batch_id" not in existing_indexes:
            op.create_index("idx_ai_tasks_batch_id", "ai_tasks", ["batch_id"])

    if not _has_table("ai_fix_batches"):
        op.create_table(
            "ai_fix_batches",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("set_id", sa.String(), nullable=False),
            sa.Column("run_mode", sa.String(), nullable=False),
            sa.Column("scope_profile", sa.String(), nullable=False),
            sa.Column("status", sa.String(), nullable=False, server_default="running"),
            sa.Column("created_by", sa.String(), nullable=True),
            sa.Column("model_name", sa.String(), nullable=True),
            sa.Column("concurrency", sa.Integer(), nullable=False, server_default="4"),
            sa.Column("max_total_cost_usd", sa.Float(), nullable=False, server_default="5"),
            sa.Column("failure_rate_stop", sa.Float(), nullable=False, server_default="0.3"),
            sa.Column("max_tasks", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("queued_count", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("running_count", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("completed_count", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("failed_count", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("applied_count", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("commit_count", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("stopped_reason", sa.Text(), nullable=True),
            sa.Column("pr_url", sa.String(), nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.CheckConstraint("run_mode IN ('pr', 'main')", name="ck_ai_fix_batches_run_mode"),
            sa.CheckConstraint(
                "scope_profile IN ('script', 'script_engine', 'script_engine_transpiler')",
                name="ck_ai_fix_batches_scope_profile",
            ),
            sa.CheckConstraint(
                "status IN ('running', 'completed', 'failed', 'failed_no_changes', 'canceled')",
                name="ck_ai_fix_batches_status",
            ),
            sa.ForeignKeyConstraint(["created_by"], ["users.id"], ondelete="SET NULL"),
            sa.PrimaryKeyConstraint("id"),
        )
        op.create_index("idx_ai_fix_batches_set_id", "ai_fix_batches", ["set_id"])
        op.create_index("idx_ai_fix_batches_status", "ai_fix_batches", ["status"])
        op.create_index("idx_ai_fix_batches_created_at", "ai_fix_batches", ["created_at"])

    if not _has_table("ai_fix_batch_items"):
        op.create_table(
            "ai_fix_batch_items",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("batch_id", sa.String(), nullable=False),
            sa.Column("issue_id", sa.String(), nullable=True),
            sa.Column("card_id", sa.String(), nullable=False),
            sa.Column("task_id", sa.String(), nullable=True),
            sa.Column("status", sa.String(), nullable=False, server_default="pending"),
            sa.Column("error_text", sa.Text(), nullable=True),
            sa.Column("applied_at", sa.DateTime(timezone=True), nullable=True),
            sa.Column("commit_sha", sa.String(), nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.CheckConstraint(
                "status IN ('pending', 'queued', 'running', 'completed', 'failed', 'applied', 'canceled')",
                name="ck_ai_fix_batch_items_status",
            ),
            sa.ForeignKeyConstraint(["batch_id"], ["ai_fix_batches.id"], ondelete="CASCADE"),
            sa.ForeignKeyConstraint(["issue_id"], ["issues.id"], ondelete="SET NULL"),
            sa.ForeignKeyConstraint(["task_id"], ["ai_tasks.id"], ondelete="SET NULL"),
            sa.PrimaryKeyConstraint("id"),
        )
        op.create_index("idx_ai_fix_batch_items_batch_id", "ai_fix_batch_items", ["batch_id"])
        op.create_index("idx_ai_fix_batch_items_card_id", "ai_fix_batch_items", ["card_id"])
        op.create_index("idx_ai_fix_batch_items_status", "ai_fix_batch_items", ["status"])

    if not _has_table("ai_fix_apply_audits"):
        op.create_table(
            "ai_fix_apply_audits",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("ai_task_id", sa.String(), nullable=True),
            sa.Column("batch_id", sa.String(), nullable=True),
            sa.Column("card_id", sa.String(), nullable=False),
            sa.Column("scope_profile", sa.String(), nullable=False),
            sa.Column("run_mode", sa.String(), nullable=False),
            sa.Column("applied_files_json", sa.Text(), nullable=False, server_default="[]"),
            sa.Column("check_outputs_json", sa.Text(), nullable=False, server_default="[]"),
            sa.Column("commit_sha", sa.String(), nullable=True),
            sa.Column("status", sa.String(), nullable=False, server_default="applied"),
            sa.Column("error_text", sa.Text(), nullable=True),
            sa.Column("created_by", sa.String(), nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.ForeignKeyConstraint(["ai_task_id"], ["ai_tasks.id"], ondelete="SET NULL"),
            sa.ForeignKeyConstraint(["batch_id"], ["ai_fix_batches.id"], ondelete="SET NULL"),
            sa.ForeignKeyConstraint(["created_by"], ["users.id"], ondelete="SET NULL"),
            sa.PrimaryKeyConstraint("id"),
        )
        op.create_index("idx_ai_fix_apply_audits_task_id", "ai_fix_apply_audits", ["ai_task_id"])
        op.create_index("idx_ai_fix_apply_audits_batch_id", "ai_fix_apply_audits", ["batch_id"])
        op.create_index("idx_ai_fix_apply_audits_card_id", "ai_fix_apply_audits", ["card_id"])
        op.create_index("idx_ai_fix_apply_audits_created_at", "ai_fix_apply_audits", ["created_at"])

    if _has_table("ai_tasks"):
        bind = op.get_bind()
        inspector = sa.inspect(bind)
        fk_names = {fk.get("name") for fk in inspector.get_foreign_keys("ai_tasks")}
        if _has_column("ai_tasks", "batch_id") and "fk_ai_tasks_batch_id" not in fk_names:
            with op.batch_alter_table("ai_tasks") as batch_op:
                batch_op.create_foreign_key(
                    "fk_ai_tasks_batch_id",
                    "ai_fix_batches",
                    ["batch_id"],
                    ["id"],
                    ondelete="SET NULL",
                )


def downgrade() -> None:
    if _has_table("ai_tasks"):
        bind = op.get_bind()
        inspector = sa.inspect(bind)
        existing_indexes = {idx["name"] for idx in inspector.get_indexes("ai_tasks")}
        if "idx_ai_tasks_batch_id" in existing_indexes:
            op.drop_index("idx_ai_tasks_batch_id", table_name="ai_tasks")

    if _has_table("ai_fix_apply_audits"):
        op.drop_index("idx_ai_fix_apply_audits_created_at", table_name="ai_fix_apply_audits")
        op.drop_index("idx_ai_fix_apply_audits_card_id", table_name="ai_fix_apply_audits")
        op.drop_index("idx_ai_fix_apply_audits_batch_id", table_name="ai_fix_apply_audits")
        op.drop_index("idx_ai_fix_apply_audits_task_id", table_name="ai_fix_apply_audits")
        op.drop_table("ai_fix_apply_audits")

    if _has_table("ai_fix_batch_items"):
        op.drop_index("idx_ai_fix_batch_items_status", table_name="ai_fix_batch_items")
        op.drop_index("idx_ai_fix_batch_items_card_id", table_name="ai_fix_batch_items")
        op.drop_index("idx_ai_fix_batch_items_batch_id", table_name="ai_fix_batch_items")
        op.drop_table("ai_fix_batch_items")

    if _has_table("ai_fix_batches"):
        op.drop_index("idx_ai_fix_batches_created_at", table_name="ai_fix_batches")
        op.drop_index("idx_ai_fix_batches_status", table_name="ai_fix_batches")
        op.drop_index("idx_ai_fix_batches_set_id", table_name="ai_fix_batches")
        op.drop_table("ai_fix_batches")

    if _has_table("ai_tasks"):
        with op.batch_alter_table("ai_tasks", recreate="always") as batch_op:
            try:
                batch_op.drop_constraint("fk_ai_tasks_batch_id", type_="foreignkey")
            except Exception:
                pass
            try:
                batch_op.drop_constraint("ck_ai_tasks_scope_profile", type_="check")
            except Exception:
                pass
            try:
                batch_op.drop_constraint("ck_ai_tasks_run_mode", type_="check")
            except Exception:
                pass
            batch_op.drop_constraint("ck_ai_tasks_task_type", type_="check")
            batch_op.create_check_constraint(
                "ck_ai_tasks_task_type",
                "task_type IN ('review_batch', 'qa_analysis', 'engine_audit')",
            )
            if _has_column("ai_tasks", "scope_profile"):
                batch_op.drop_column("scope_profile")
            if _has_column("ai_tasks", "run_mode"):
                batch_op.drop_column("run_mode")
            if _has_column("ai_tasks", "batch_id"):
                batch_op.drop_column("batch_id")
