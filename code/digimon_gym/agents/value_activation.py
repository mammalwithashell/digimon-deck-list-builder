"""Output activation of the search value head.

SB3's ``value_net`` is a bare ``Linear`` and its state-dict keys are fixed, so a
``tanh`` cannot be added as a layer without breaking every existing checkpoint
load. The activation is instead a checkpoint attribute
(``digimon_value_activation``) applied by every consumer that turns the critic
latent into a value: the AlphaZero trainer, value calibration, and the ONNX
export (which bakes it into the graph, so native search needs no change).

``identity`` is the legacy behaviour: an unbounded output the engine clamps to
[-1, 1]. ``tanh`` bounds the output smoothly, so saturated readings stop
arriving as flat clipped values. A checkpoint converts identity -> tanh once;
the reverse is refused because the weights were fitted through the squash.
"""
from __future__ import annotations

ACTIVATIONS = ("identity", "tanh")
ATTRIBUTE = "digimon_value_activation"


def get(model) -> str:
    name = getattr(model, ATTRIBUTE, "identity") or "identity"
    if name not in ACTIVATIONS:
        raise ValueError(f"unknown value activation {name!r} on checkpoint")
    return name


def resolve(model, requested: str | None) -> str:
    """Return the activation to train with and record it on ``model``."""
    current = get(model)
    if requested is None:
        return current
    if requested not in ACTIVATIONS:
        raise ValueError(f"unknown value activation {requested!r}; expected one of {ACTIVATIONS}")
    if current == "tanh" and requested == "identity":
        raise ValueError("checkpoint value head was fitted through tanh; refusing to drop it")
    setattr(model, ATTRIBUTE, requested)
    return requested


def apply(name: str, values):
    """Apply the activation to a torch tensor of raw ``value_net`` outputs."""
    if name == "identity":
        return values
    if name == "tanh":
        import torch
        return torch.tanh(values)
    raise ValueError(f"unknown value activation {name!r}")
