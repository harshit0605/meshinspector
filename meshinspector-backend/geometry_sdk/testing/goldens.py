"""Stored golden reference helpers for parity tests."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import numpy as np


GOLDEN_DIR = Path(__file__).resolve().parent / "golden_data"


def load_golden(name: str) -> dict[str, Any]:
    path = GOLDEN_DIR / name
    return json.loads(path.read_text(encoding="utf-8"))


def assert_close(actual: float | None, expected: float | None, *, abs_tol: float = 1e-6, rel_tol: float = 0.0) -> None:
    if actual is None or expected is None:
        assert actual is expected
        return
    assert np.isclose(float(actual), float(expected), atol=abs_tol, rtol=rel_tol), f"{actual!r} != {expected!r}"


def assert_metric_dict_close(
    actual: dict[str, Any],
    expected: dict[str, Any],
    *,
    abs_tol: float = 1e-6,
    rel_tol: float = 0.0,
    keys: list[str] | None = None,
) -> None:
    keys = keys or list(expected.keys())
    for key in keys:
        if isinstance(expected[key], float) or expected[key] is None:
            assert_close(actual.get(key), expected[key], abs_tol=abs_tol, rel_tol=rel_tol)
        else:
            assert actual.get(key) == expected[key]
