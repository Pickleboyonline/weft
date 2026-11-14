"""Behavioral models for externs.

Every escape hatch must say how it behaves in simulation. The simulator runs the
model in place of the real implementation, with randomness drawn from the run's
seed, so failures are reproducible.
"""

from __future__ import annotations

from typing import Any


def fallible(*, fail_rate: float, latency_ms: tuple[int, int] = (0, 0), ok: Any = True, err: Any = False) -> dict:
    """Returns `ok`, or `err` with probability `fail_rate`, after a uniform latency."""
    return {"model": "fallible", "fail_rate": fail_rate, "latency_ms": list(latency_ms), "ok": ok, "err": err}


def constant(value: Any, *, latency_ms: tuple[int, int] = (0, 0)) -> dict:
    """Always returns `value`, after a uniform latency."""
    return {"model": "constant", "value": value, "latency_ms": list(latency_ms)}
