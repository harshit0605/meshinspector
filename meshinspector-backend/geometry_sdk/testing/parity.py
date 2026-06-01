"""Parity assertions shared by SDK tests."""

from __future__ import annotations

from dataclasses import asdict, is_dataclass
from typing import Any

import numpy as np


def numeric_close(actual: float | None, expected: float | None, *, abs_tol: float, rel_tol: float = 0.0) -> bool:
    if actual is None or expected is None:
        return actual is expected
    return bool(np.isclose(float(actual), float(expected), atol=abs_tol, rtol=rel_tol))


def dataclass_to_plain(value: Any) -> Any:
    if is_dataclass(value):
        return asdict(value)
    return value
