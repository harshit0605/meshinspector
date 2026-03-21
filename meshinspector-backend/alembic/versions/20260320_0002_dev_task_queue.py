"""database-backed development queue

Revision ID: 20260320_0002
Revises: 20260320_0001
Create Date: 2026-03-20 18:05:00
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260320_0002"
down_revision = "20260320_0001"
branch_labels = None
depends_on = None


def upgrade() -> None:
    bind = op.get_bind()
    if bind.dialect.name == "postgresql":
        op.execute(
            """
            CREATE UNLOGGED TABLE dev_task_queue (
                id VARCHAR(32) PRIMARY KEY,
                task_name VARCHAR(64) NOT NULL,
                job_id VARCHAR(32) UNIQUE NULL REFERENCES jobs(id),
                payload_json JSON NOT NULL,
                status VARCHAR(32) NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
                available_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                locked_at TIMESTAMPTZ NULL,
                locked_by VARCHAR(128) NULL,
                error_message TEXT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            """
        )
    else:
        op.create_table(
            "dev_task_queue",
            sa.Column("id", sa.String(length=32), nullable=False),
            sa.Column("task_name", sa.String(length=64), nullable=False),
            sa.Column("job_id", sa.String(length=32), nullable=True),
            sa.Column("payload_json", sa.JSON(), nullable=False),
            sa.Column("status", sa.String(length=32), nullable=False),
            sa.Column("attempts", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("available_at", sa.DateTime(timezone=True), server_default=sa.text("CURRENT_TIMESTAMP"), nullable=False),
            sa.Column("locked_at", sa.DateTime(timezone=True), nullable=True),
            sa.Column("locked_by", sa.String(length=128), nullable=True),
            sa.Column("error_message", sa.Text(), nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("CURRENT_TIMESTAMP"), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), server_default=sa.text("CURRENT_TIMESTAMP"), nullable=False),
            sa.ForeignKeyConstraint(["job_id"], ["jobs.id"]),
            sa.PrimaryKeyConstraint("id"),
            sa.UniqueConstraint("job_id"),
        )

    op.create_index("ix_dev_task_queue_available_at", "dev_task_queue", ["available_at"])
    op.create_index("ix_dev_task_queue_job_id", "dev_task_queue", ["job_id"], unique=True)
    op.create_index("ix_dev_task_queue_status", "dev_task_queue", ["status"])
    op.create_index("ix_dev_task_queue_task_name", "dev_task_queue", ["task_name"])


def downgrade() -> None:
    op.drop_index("ix_dev_task_queue_task_name", table_name="dev_task_queue")
    op.drop_index("ix_dev_task_queue_status", table_name="dev_task_queue")
    op.drop_index("ix_dev_task_queue_job_id", table_name="dev_task_queue")
    op.drop_index("ix_dev_task_queue_available_at", table_name="dev_task_queue")
    op.drop_table("dev_task_queue")
