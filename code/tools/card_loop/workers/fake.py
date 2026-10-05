"""`FakeWorker`: replays canned `WorkerResult`s through the real worker contract.

The driver's test double (spec "Driver tests without a model"). Responses are
looked up per packet, most specific first:

    (stage, item)  ->  item  ->  stage  ->  the ordered queue (a list)

A mapping value may be a `WorkerResult`, a list of them (consumed one per call,
so a schema retry or a second attempt gets the next one), or a callable
`fn(packet) -> WorkerResult`. Every received packet is recorded in `received`.
Running out of responses raises `LookupError` unless `default` is set: a driver
test that calls a worker more often than it scripted is a bug in the test or
the driver, never something to paper over.
"""
from __future__ import annotations

import threading
from collections import deque
from typing import Callable, Iterable, Mapping, Union

from ..contracts import FAMILIES, TaskPacket, WorkerResult

Response = Union[WorkerResult, Callable[[TaskPacket], WorkerResult]]


class FakeWorker:
    def __init__(
        self,
        responses: Union[Iterable[Response], Mapping[object, Union[Response, list]]] = (),
        *,
        family: str = "claude",
        default: Response | None = None,
    ):
        if family not in FAMILIES:
            raise ValueError(f"unknown family {family!r}")
        self.family = family
        self.default = default
        self.received: list[TaskPacket] = []
        self._lock = threading.Lock()
        self._keyed: dict[object, deque] = {}
        self._sticky: dict[object, Callable[[TaskPacket], WorkerResult]] = {}
        self._queue: deque = deque()
        if isinstance(responses, Mapping):
            for key, value in responses.items():
                if isinstance(value, list):
                    self._keyed[key] = deque(value)
                elif callable(value) and not isinstance(value, WorkerResult):
                    self._sticky[key] = value  # a callable answers every call for its key
                else:
                    self._keyed[key] = deque([value])
        else:
            self._queue.extend(responses)

    @property
    def calls(self) -> int:
        return len(self.received)

    def packets_for(self, *, stage: str | None = None, item: str | None = None) -> list[TaskPacket]:
        return [p for p in self.received
                if (stage is None or p.stage == stage) and (item is None or p.item == item)]

    def run(self, packet: TaskPacket) -> WorkerResult:
        if packet.family != self.family:
            raise ValueError(f"packet for {packet.family!r} sent to fake {self.family!r} worker")
        with self._lock:
            self.received.append(packet)
            response = self._next(packet)
        if response is None:
            raise LookupError(f"FakeWorker({self.family}) has no response left for "
                              f"stage={packet.stage!r} item={packet.item!r} "
                              f"(call #{len(self.received)})")
        result = response(packet) if callable(response) and not isinstance(response, WorkerResult) else response
        if not isinstance(result, WorkerResult):
            raise TypeError(f"fake response must be a WorkerResult, got {type(result).__name__}")
        return result

    def _next(self, packet: TaskPacket):
        for key in ((packet.stage, packet.item), packet.item, packet.stage):
            if key in self._sticky:
                return self._sticky[key]
            dq = self._keyed.get(key)
            if dq:
                return dq.popleft()
        if self._queue:
            return self._queue.popleft()
        return self.default
