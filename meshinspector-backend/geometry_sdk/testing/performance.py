"""Small performance-budget helpers for accelerator parity tests."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
import os
from time import perf_counter
from typing import TypeVar


T = TypeVar("T")


@dataclass(frozen=True, slots=True)
class PerformanceSample:
    python_seconds: float
    rust_seconds: float
    ratio: float


def best_of(repeats: int, callback: Callable[[], T]) -> tuple[float, T]:
    best_seconds = float("inf")
    best_value: T | None = None
    for _ in range(repeats):
        started = perf_counter()
        value = callback()
        elapsed = perf_counter() - started
        if elapsed < best_seconds:
            best_seconds = elapsed
            best_value = value
    assert best_value is not None
    return best_seconds, best_value


def compare_accelerator_modes(callback: Callable[[], T], *, repeats: int = 3) -> tuple[PerformanceSample, T, T]:
    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "python"
        callback()
        python_seconds, python_value = best_of(repeats, callback)

        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        callback()
        rust_seconds, rust_value = best_of(repeats, callback)
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    return (
        PerformanceSample(
            python_seconds=python_seconds,
            rust_seconds=rust_seconds,
            ratio=rust_seconds / max(python_seconds, 1e-12),
        ),
        python_value,
        rust_value,
    )
