"""Make dev_task_queue logged on PostgreSQL.

Revision ID: 20260530_0003
Revises: 20260320_0002
Create Date: 2026-05-30 18:30:00
"""

from __future__ import annotations

from alembic import op


revision = "20260530_0003"
down_revision = "20260320_0002"
branch_labels = None
depends_on = None


def upgrade() -> None:
    bind = op.get_bind()
    if bind.dialect.name == "postgresql":
        op.execute("ALTER TABLE dev_task_queue SET LOGGED")


def downgrade() -> None:
    bind = op.get_bind()
    if bind.dialect.name == "postgresql":
        op.execute("ALTER TABLE dev_task_queue SET UNLOGGED")
