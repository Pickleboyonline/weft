"""Tracing front end: Python functions in, Weft IR v0 (a JSON-able dict) out."""

from __future__ import annotations

import contextlib
import dataclasses
import functools
import inspect
import json
from pathlib import Path
from typing import Any, Callable, Iterator

IR_VERSION = 0


class TracingError(Exception):
    """The program can't be expressed in Weft IR as written."""


# ------------------------------------------------------------------ types


class Type:
    def __init__(self, name: str) -> None:
        self.name = name

    def __repr__(self) -> str:
        return f"weft.{self.name.capitalize()}"


Int = Type("int")
Bool = Type("bool")
Loc = Type("loc")  # one replica of a location, e.g. weft.sender()


# ------------------------------------------------------------------ trace state


class _Trace:
    """The op list being recorded for one handler or property."""

    def __init__(self, what: str, location: Location | None, message: type[Message] | None = None, *, sender: bool = False):
        self.what = what
        self.location = location  # None means a property: pure, may read any location
        self.message = message
        self.sender = sender
        self.ops: list[dict[str, Any]] = []
        self.guards: list[int] = []  # cumulative guard op ids from nested `with when(...)`

    def emit(self, op: str, ty: Type | None = None, *, effect: bool = False, **attrs: Any) -> int:
        if effect and self.location is None:
            raise TracingError(f"{self.what}: properties are pure; they cannot {op}")
        node: dict[str, Any] = {"id": len(self.ops), "op": op}
        if ty is not None:
            node["type"] = ty.name
        node.update(attrs)
        if effect and self.guards:
            node["guard"] = self.guards[-1]
        self.ops.append(node)
        return node["id"]

    def value(self, op: str, ty: Type, **attrs: Any) -> Value:
        return Value(self, self.emit(op, ty, **attrs), ty)


_current: _Trace | None = None


def _trace(what: str) -> _Trace:
    if _current is None:
        raise TracingError(f"{what} only works inside a handler or property while it is being traced")
    return _current


@contextlib.contextmanager
def _tracing(t: _Trace) -> Iterator[_Trace]:
    global _current
    prev, _current = _current, t
    try:
        yield t
    finally:
        _current = prev


# ------------------------------------------------------------------ traced values

_CONTROL_FLOW_HELP = """a traced value was used in Python control flow (if / while / and / or / not / bool()).
Python runs once, at build time, to record the program's structure; it never sees runtime values.
  conditional effects:  with weft.when(cond): ...
  conditional values:   weft.select(cond, a, b)
  boolean logic:        a & b,  a | b,  ~a"""

_ARITH = {"add", "sub", "mul"}
_ORDER = {"lt", "le", "gt", "ge"}
_LOGIC = {"and", "or"}


class Value:
    """A traced value: a reference to an op's result, never a runtime value."""

    __slots__ = ("_t", "id", "type")

    def __init__(self, t: _Trace, id: int, type: Type) -> None:
        self._t, self.id, self.type = t, id, type

    def __bool__(self) -> bool:
        raise TracingError(_CONTROL_FLOW_HELP)

    def __repr__(self) -> str:
        return f"<traced {self.type.name} %{self.id}>"

    def _bin(self, fn: str, other: Any, reverse: bool = False) -> Value:
        o = _lift(other, self.type if fn not in _LOGIC else Bool, self._t)
        a, b = (o, self) if reverse else (self, o)
        want = Int if fn in _ARITH | _ORDER else Bool if fn in _LOGIC else None
        if want is not None and (a.type is not want or b.type is not want):
            raise TracingError(f"`{fn}` needs {want.name} operands, got {a.type.name} and {b.type.name}")
        if want is None and a.type is not b.type:
            raise TracingError(f"`{fn}` compares {a.type.name} with {b.type.name}")
        return self._t.value("bin", Int if fn in _ARITH else Bool, fn=fn, args=[a.id, b.id])

    def __add__(self, o: Any) -> Value: return self._bin("add", o)
    def __radd__(self, o: Any) -> Value: return self._bin("add", o, True)
    def __sub__(self, o: Any) -> Value: return self._bin("sub", o)
    def __rsub__(self, o: Any) -> Value: return self._bin("sub", o, True)
    def __mul__(self, o: Any) -> Value: return self._bin("mul", o)
    def __rmul__(self, o: Any) -> Value: return self._bin("mul", o, True)
    def __eq__(self, o: Any) -> Value: return self._bin("eq", o)  # type: ignore[override]
    def __ne__(self, o: Any) -> Value: return self._bin("ne", o)  # type: ignore[override]
    def __lt__(self, o: Any) -> Value: return self._bin("lt", o)
    def __le__(self, o: Any) -> Value: return self._bin("le", o)
    def __gt__(self, o: Any) -> Value: return self._bin("gt", o)
    def __ge__(self, o: Any) -> Value: return self._bin("ge", o)
    def __and__(self, o: Any) -> Value: return self._bin("and", o)
    def __rand__(self, o: Any) -> Value: return self._bin("and", o, True)
    def __or__(self, o: Any) -> Value: return self._bin("or", o)
    def __ror__(self, o: Any) -> Value: return self._bin("or", o, True)

    def __invert__(self) -> Value:
        if self.type is not Bool:
            raise TracingError(f"`~` needs a bool, got {self.type.name}")
        return self._t.value("not", Bool, args=[self.id])


def _lift(x: Any, ty: Type | None, t: _Trace) -> Value:
    """Turn a Python constant into a const op; pass traced values through."""
    if isinstance(x, Value):
        if x._t is not t:
            raise TracingError(f"{x!r} comes from a different handler; values can't cross handlers (send a message)")
        if ty is not None and x.type is not ty:
            raise TracingError(f"expected {ty.name}, got {x.type.name}")
        return x
    if isinstance(x, bool):
        cty = Bool
    elif isinstance(x, int):
        cty = Int
    else:
        raise TracingError(f"can't use {x!r} ({type(x).__name__}) in a Weft program; v0 has int and bool constants")
    if ty is not None and cty is not ty:
        raise TracingError(f"expected {ty.name}, got the constant {x!r}")
    return t.value("const", cty, value=x)


def select(cond: Any, if_true: Any, if_false: Any) -> Value:
    """`if_true` when `cond` holds, else `if_false`. Both sides are always computed."""
    t = _trace("weft.select")
    c = _lift(cond, Bool, t)
    hint = next((v.type for v in (if_true, if_false) if isinstance(v, Value)), None)
    a, b = _lift(if_true, hint, t), _lift(if_false, hint, t)
    if a.type is not b.type:
        raise TracingError(f"weft.select branches differ: {a.type.name} vs {b.type.name}")
    return t.value("select", a.type, args=[c.id, a.id, b.id])


@contextlib.contextmanager
def when(cond: Any) -> Iterator[None]:
    """Effects (state writes, sends, extern calls) in this block happen only if `cond` is true.

    Pure values computed inside are not affected and can be used after the block.
    A guarded-off extern call yields its type's zero value (0 / False).
    """
    t = _trace("weft.when")
    c = _lift(cond, Bool, t)
    g = t.emit("bin", Bool, fn="and", args=[t.guards[-1], c.id]) if t.guards else c.id
    t.guards.append(g)
    try:
        yield
    finally:
        t.guards.pop()


def unless(cond: Any) -> contextlib.AbstractContextManager[None]:
    return when(~_lift(cond, Bool, _trace("weft.unless")))


def sender() -> Value:
    """The replica that sent the message being handled."""
    t = _trace("weft.sender()")
    if not t.sender:
        raise TracingError(f"{t.what}: weft.sender() exists only in handlers for network messages (@loc.on)")
    return t.value("sender", Loc)


def send(to: Location | Value, message: Message) -> None:
    """Send to every replica of a location, or to one replica given as a loc value."""
    t = _trace("weft.send")
    if t.location is None:
        raise TracingError(f"{t.what}: properties are pure; they cannot send")
    if not isinstance(message, Message):
        raise TracingError(f"weft.send needs a Message instance, got {message!r}")
    t.location.app._register_message(type(message))
    if isinstance(to, Location):
        target: dict[str, Any] = {"location": to.name}
    elif isinstance(to, Value) and to.type is Loc:
        target = {"value": _lift(to, Loc, t).id}
    else:
        raise TracingError(f"send target must be a location or a loc value (e.g. weft.sender()), got {to!r}")
    fields = {name: _lift(message._values[name], ty, t).id for name, ty in type(message)._fields.items()}
    t.emit("send", effect=True, to=target, message=type(message).__name__, fields=fields)


# ------------------------------------------------------------------ messages


class Message:
    """A typed record that moves between locations.

        class Offer(weft.Message):
            version: weft.Int
    """

    _fields: dict[str, Type] = {}

    def __init_subclass__(cls, **kw: Any) -> None:
        super().__init_subclass__(**kw)
        fields: dict[str, Type] = {}
        for name, ann in inspect.get_annotations(cls, eval_str=True).items():
            if not isinstance(ann, Type):
                raise TracingError(f"{cls.__name__}.{name}: field types are weft.Int / weft.Bool / weft.Loc, got {ann!r}")
            fields[name] = ann
        cls._fields = fields

    def __init__(self, **values: Any) -> None:
        missing = sorted(self._fields.keys() - values.keys())
        extra = sorted(values.keys() - self._fields.keys())
        if missing or extra:
            raise TracingError(f"{type(self).__name__}(...): missing fields {missing}, unexpected fields {extra}")
        self._values = values

    def __repr__(self) -> str:
        return f"{type(self).__name__}({', '.join(f'{k}={v!r}' for k, v in self._values.items())})"


class _Incoming:
    """The triggering message inside a handler: field access emits `field` ops."""

    def __init__(self, cls: type[Message], t: _Trace) -> None:
        self._cls, self._t, self._cache = cls, t, {}

    def __getattr__(self, name: str) -> Value:
        if name.startswith("_"):
            raise AttributeError(name)
        ty = self._cls._fields.get(name)
        if ty is None:
            raise TracingError(f"{self._cls.__name__} has no field `{name}`")
        if name not in self._cache:
            self._cache[name] = self._t.value("field", ty, name=name)
        return self._cache[name]


# ------------------------------------------------------------------ program structure


class State:
    """A durable, typed cell owned by one location. Read and write it only from that location."""

    def __init__(self, location: Location, name: str, type: Type, init: Any) -> None:
        if not isinstance(type, Type):
            raise TracingError(f"state `{name}`: type must be weft.Int / weft.Bool, got {type!r}")
        ok = isinstance(init, bool) if type is Bool else isinstance(init, int) and not isinstance(init, bool)
        if type is Loc or not ok:
            raise TracingError(f"state `{name}`: initial value {init!r} is not a {type.name}")
        self.location, self.name, self.type, self.init = location, name, type, init
        self.qualname = f"{location.name}.{name}"

    def _check_local(self, t: _Trace, verb: str) -> None:
        if t.location is not None and t.location is not self.location:
            raise TracingError(
                f"{t.what} runs on `{t.location.name}` but tries to {verb} `{self.qualname}`, "
                f"which lives on `{self.location.name}`.\nLocations share no memory; send a message instead."
            )

    def read(self) -> Value:
        t = _trace(f"{self.qualname}.read()")
        self._check_local(t, "read")
        return t.value("read", self.type, state=self.qualname)

    def write(self, value: Any) -> None:
        t = _trace(f"{self.qualname}.write()")
        self._check_local(t, "write")
        v = _lift(value, self.type, t)
        t.emit("write", effect=True, state=self.qualname, args=[v.id])

    def __repr__(self) -> str:
        return f"<state {self.qualname}: {self.type.name}>"


class Extern:
    """An escape hatch: code Weft can't see into, with declared effects and a required sim model."""

    def __init__(self, location: Location, fn: Callable[..., Any], effects: list[str], sim: dict | None) -> None:
        name = fn.__name__
        if sim is None:
            raise TracingError(
                f"extern `{name}` needs sim=...: every escape hatch must say how it behaves in simulation (see weft.sim)"
            )
        hints = inspect.get_annotations(fn, eval_str=True)
        params = []
        for p in inspect.signature(fn).parameters:
            ty = hints.get(p)
            if not isinstance(ty, Type):
                raise TracingError(f"extern `{name}`: parameter `{p}` needs a weft type annotation")
            params.append((p, ty))
        returns = hints.get("return")
        if not isinstance(returns, Type):
            raise TracingError(f"extern `{name}`: needs a weft return type annotation (-> weft.Bool)")
        self.location, self.name, self.params, self.returns = location, name, params, returns
        self.decl = {
            "name": name,
            "location": location.name,
            "params": [[p, ty.name] for p, ty in params],
            "returns": returns.name,
            "effects": list(effects),
            "sim": sim,
        }
        location.app._externs.append(self.decl)
        functools.update_wrapper(self, fn)

    def __call__(self, *args: Any) -> Value:
        t = _trace(f"extern `{self.name}`")
        if t.location is None:
            raise TracingError(f"{t.what}: properties are pure; they cannot call externs")
        if t.location is not self.location:
            raise TracingError(f"{t.what} runs on `{t.location.name}`; extern `{self.name}` only exists on `{self.location.name}`")
        if len(args) != len(self.params):
            raise TracingError(f"extern `{self.name}` takes {len(self.params)} args, got {len(args)}")
        ids = [_lift(a, ty, t).id for a, (_, ty) in zip(args, self.params)]
        return t.value("call", self.returns, effect=True, extern=self.name, args=ids)


class Location:
    """A place code runs: a device class, an edge node, a cloud service.

    Defined by its capabilities, not by its kind. `replicas` is how many copies
    exist (5 devices, 1 cloud); each has its own state.
    """

    def __init__(self, app: App, name: str, replicas: int, caps: list[str]) -> None:
        self.app, self.name, self.replicas, self.caps = app, name, replicas, caps

    def state(self, name: str, type: Type, *, init: Any) -> State:
        s = State(self, name, type, init)
        self.app._state.append(s)
        return s

    def extern(self, *, effects: list[str], sim: dict | None = None) -> Callable[[Callable[..., Any]], Extern]:
        return lambda fn: Extern(self, fn, effects, sim)

    def on(self, message: type[Message]) -> Callable[[Callable], Callable]:
        """Handle a message from another location. `weft.sender()` is available."""
        return self._handler({"kind": "message", "message": message.__name__}, message, sender=True)

    def on_input(self, message: type[Message]) -> Callable[[Callable], Callable]:
        """Handle a message from outside the system (an operator, a user, a test scenario)."""
        return self._handler({"kind": "input", "message": message.__name__}, message, sender=False)

    def every(self, *, ms: int) -> Callable[[Callable], Callable]:
        """Run periodically. Needs the `clock` capability."""
        return self._handler({"kind": "timer", "every_ms": ms}, None, sender=False)

    def _handler(self, trigger: dict, message: type[Message] | None, *, sender: bool) -> Callable[[Callable], Callable]:
        def deco(fn: Callable) -> Callable:
            if message is not None:
                self.app._register_message(message)
            t = _Trace(f"handler `{fn.__name__}`", self, message, sender=sender)
            with _tracing(t):
                ret = fn(_Incoming(message, t)) if message is not None else fn()
            if ret is not None:
                raise TracingError(f"{t.what} returned a value; handlers communicate with weft.send(...)")
            self.app._handlers.append({"name": fn.__name__, "location": self.name, "trigger": trigger, "body": t.ops})
            return fn

        return deco

    def __repr__(self) -> str:
        return f"<location {self.name} x{self.replicas}>"


@dataclasses.dataclass(frozen=True)
class Input:
    at_ms: int
    location: Location
    message: Message
    replica: int | None = None


def at(ms: int, location: Location, message: Message, *, replica: int | None = None) -> Input:
    """A scenario input: deliver `message` to `location` from outside at time `ms`."""
    return Input(ms, location, message, replica)


def network(*, drop_rate: float = 0.0, latency_ms: tuple[int, int] = (1, 1)) -> dict:
    """Simulated network: each message is dropped with `drop_rate` and delayed uniformly."""
    return {"drop_rate": drop_rate, "latency_ms": list(latency_ms)}


class App:
    """A whole system: every location, the code on each, and how to test it."""

    def __init__(self, name: str) -> None:
        self.name = name
        self._locations: list[Location] = []
        self._messages: dict[str, type[Message]] = {}
        self._state: list[State] = []
        self._externs: list[dict] = []
        self._handlers: list[dict] = []
        self._properties: list[dict] = []
        self._scenarios: list[dict] = []

    def location(self, name: str, *, replicas: int = 1, caps: list[str] | tuple[str, ...] = ()) -> Location:
        if any(l.name == name for l in self._locations):
            raise TracingError(f"location `{name}` is defined twice")
        loc = Location(self, name, replicas, list(caps))
        self._locations.append(loc)
        return loc

    def _register_message(self, cls: Any) -> None:
        if not (isinstance(cls, type) and issubclass(cls, Message)):
            raise TracingError(f"expected a weft.Message subclass, got {cls!r}")
        existing = self._messages.setdefault(cls.__name__, cls)
        if existing is not cls:
            raise TracingError(f"two different message classes are named {cls.__name__}")

    # -- properties: what the simulator checks

    def always(self, fn: Callable[[], Any]) -> Callable[[], Any]:
        """Must hold after every event (for every replica of the location it reads)."""
        self._property("always", fn)
        return fn

    def eventually(self, fn: Callable[[], Any]) -> Callable[[], Any]:
        """Must hold at the end of the run, after faults heal."""
        self._property("eventually", fn)
        return fn

    def monotonic(self, state: State, *, name: str | None = None) -> None:
        """An int state never decreases, on any replica."""
        if state.type is not Int:
            raise TracingError(f"monotonic needs an int state, got {state!r}")
        self._properties.append({"name": name or f"{state.name}_monotonic", "kind": "monotonic", "state": state.qualname})

    def _property(self, kind: str, fn: Callable[[], Any]) -> None:
        t = _Trace(f"property `{fn.__name__}`", None)
        with _tracing(t):
            result = _lift(fn(), Bool, t)
        by_name = {s.qualname: s for s in self._state}
        replicated = sorted(
            {
                by_name[op["state"]].location.name
                for op in t.ops
                if op["op"] == "read" and by_name[op["state"]].location.replicas > 1
            }
        )
        if len(replicated) > 1:
            raise TracingError(f"{t.what} reads several replicated locations ({', '.join(replicated)}); v0 quantifies over one")
        self._properties.append(
            {
                "name": fn.__name__,
                "kind": kind,
                "forall": replicated[0] if replicated else None,
                "body": t.ops,
                "result": result.id,
            }
        )

    # -- scenarios: test harness, never deployed

    def scenario(
        self,
        name: str,
        *,
        duration_ms: int,
        inputs: list[Input] = (),  # type: ignore[assignment]
        network: dict | None = None,
        heal_at_ms: int | None = None,
    ) -> None:
        ins = []
        for i in inputs:
            self._register_message(type(i.message))
            for k, v in i.message._values.items():
                if isinstance(v, Value):
                    raise TracingError(f"scenario `{name}`: input field `{k}` must be a constant")
            ins.append(
                {
                    "at_ms": i.at_ms,
                    "location": i.location.name,
                    "replica": i.replica,
                    "message": type(i.message).__name__,
                    "fields": dict(i.message._values),
                }
            )
        self._scenarios.append(
            {
                "name": name,
                "duration_ms": duration_ms,
                "heal_at_ms": heal_at_ms,
                "network": network or {"drop_rate": 0.0, "latency_ms": [1, 1]},
                "inputs": ins,
            }
        )

    # -- output

    def to_ir(self) -> dict[str, Any]:
        return {
            "weft_ir": IR_VERSION,
            "name": self.name,
            "locations": [{"name": l.name, "replicas": l.replicas, "caps": l.caps} for l in self._locations],
            "messages": [
                {"name": n, "fields": [[f, ty.name] for f, ty in cls._fields.items()]} for n, cls in self._messages.items()
            ],
            "state": [
                {"name": s.qualname, "location": s.location.name, "type": s.type.name, "init": s.init} for s in self._state
            ],
            "externs": self._externs,
            "handlers": self._handlers,
            "properties": self._properties,
            "scenarios": self._scenarios,
        }

    def save(self, path: str | Path) -> Path:
        p = Path(path)
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(self.to_ir(), indent=2) + "\n")
        return p
