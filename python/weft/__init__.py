"""Weft: write distributed-system logic as traced Python; get a graph IR out.

Like JAX, the Python you write never runs in production. It runs once, at build
time, to *trace* each handler into a graph of ops (Weft IR v0). The simulator and
(later) the backends consume that IR. Python is the metaprogramming layer: plain
`if`/`for` on build-time values choose which graph to build; traced values flow
through `weft.when`, `weft.select` and operators.
"""

from . import sim
from .core import (
    App,
    Bool,
    Extern,
    Input,
    Int,
    Loc,
    Location,
    Message,
    State,
    TracingError,
    Type,
    Value,
    at,
    network,
    select,
    send,
    sender,
    unless,
    when,
)

__all__ = [
    "App",
    "Bool",
    "Extern",
    "Input",
    "Int",
    "Loc",
    "Location",
    "Message",
    "State",
    "TracingError",
    "Type",
    "Value",
    "at",
    "network",
    "select",
    "send",
    "sender",
    "sim",
    "unless",
    "when",
]
