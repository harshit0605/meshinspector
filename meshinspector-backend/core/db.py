"""Database setup and session helpers."""

from __future__ import annotations

from collections.abc import Generator

from sqlalchemy import create_engine
from sqlalchemy.orm import DeclarativeBase, Session, sessionmaker

from core.config import settings


class Base(DeclarativeBase):
    """Base class for ORM models."""


database_url = settings.effective_database_url
is_sqlite = database_url.startswith("sqlite")
connect_args = {"check_same_thread": False} if is_sqlite else {}

engine = create_engine(
    database_url,
    echo=settings.DATABASE_ECHO,
    future=True,
    connect_args=connect_args,
    pool_pre_ping=not is_sqlite,
    pool_recycle=300 if not is_sqlite else -1,
)
SessionLocal = sessionmaker(bind=engine, autoflush=False, autocommit=False, expire_on_commit=False)


def get_db() -> Generator[Session, None, None]:
    """Yield a scoped database session."""
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()
