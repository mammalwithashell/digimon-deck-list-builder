"""baseline schema marker

Revision ID: 20260222_0001
Revises:
Create Date: 2026-02-22 00:00:00
"""

from __future__ import annotations

from alembic import op


# revision identifiers, used by Alembic.
revision = "20260222_0001"
down_revision = None
branch_labels = None
depends_on = None


def upgrade() -> None:
    # Baseline marker. Existing deployments are already managed by SQLAlchemy
    # create_all; future revisions should be added after this baseline.
    pass


def downgrade() -> None:
    pass
