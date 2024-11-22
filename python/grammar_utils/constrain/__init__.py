from functools import reduce

import numpy as np

from grammar_utils._internal import LR1Constraint, RegexConstraint  # noqa
from grammar_utils.grammars import load_grammar_and_lexer


def load_lr1_constraint(
    name: str,
    vocab: list[list[int]],
    exact: bool = False,
    lru_cache_size: int | None = None,
) -> LR1Constraint:
    """

    Load a LR(1) constraint for the given name.
    Currently supported:
    - json
    - sparql

    """
    return LR1Constraint(
        *load_grammar_and_lexer(name), vocab, exact=exact, lru_cache_size=lru_cache_size
    )


def load_regex_constraint(name: str, vocab: list[list[int]]) -> RegexConstraint:
    """

    Load a regex constraint for the given name.
    Currently supported:
    - boolean
    - integer
    - decimal

    """
    if name == "boolean":
        regex = "true|false"
    elif name == "integer":
        regex = "[+-]?\d+"
    elif name == "decimal":
        regex = "[+-]?(\d+(\.\d*)?|\.\d+)"
    else:
        raise ValueError(f"unsupported regex constraint: {name}")
    return RegexConstraint(regex, vocab)


class Constraint:
    """
    Base class for constraints.
    """

    def get(self) -> np.ndarray:
        """
        Returns the current constraint indices.
        """
        raise NotImplementedError

    def reset(self, input: bytes | None = None) -> None:
        """
        Resets the constraint to the initial state.
        """
        raise NotImplementedError

    def next(self, index: int) -> None:
        """
        Updates the constraint based on the chosen index / token id.
        """
        raise NotImplementedError

    def is_match(self) -> bool:
        """
        Returns whether the current state matches the constraint.
        """
        raise NotImplementedError

    def is_invalid(self) -> bool:
        """
        Returns whether the current state is invalid.
        This must be true iff get() returns an empty list of indices in
        a non-match state.
        We have a separate function for that because depending on the constraint
        this can be implemented more efficiently.
        """
        return not self.is_match() and len(self.get()) == 0

    def clone(self) -> "Constraint":
        """
        Returns a copy of the constraint.
        """
        raise NotImplementedError


class AndConstraint(Constraint):
    """

    A constraint that is the intersection of multiple constraints.

    """

    def __init__(self, constraints: list[Constraint]):
        assert len(constraints) > 0, "at least one constraint required"
        self.constraints = constraints

    def get(self) -> np.ndarray:
        return reduce(np.intersect1d, (c.get() for c in self.constraints))

    def reset(self, input: bytes | None = None) -> None:
        for c in self.constraints:
            c.reset(input)

    def next(self, index: int) -> None:
        for c in self.constraints:
            c.next(index)

    def is_match(self) -> bool:
        return all(c.is_match() for c in self.constraints)

    def clone(self) -> "AndConstraint":
        return AndConstraint([c.clone() for c in self.constraints])


class OrConstraint(Constraint):
    """

    A constraint that is the union of multiple constraints.

    """

    def __init__(self, constraints: list[Constraint]):
        assert len(constraints) > 0, "at least one constraint required"
        self.constraints = constraints

    def get(self) -> np.ndarray:
        return reduce(np.union1d, (c.get() for c in self.constraints))

    def reset(self, input: bytes | None = None) -> None:
        for c in self.constraints:
            c.reset(input)

    def next(self, index: int) -> None:
        for c in self.constraints:
            c.next(index)

    def is_match(self) -> bool:
        return any(c.is_match() for c in self.constraints)

    def clone(self) -> "OrConstraint":
        return OrConstraint([c.clone() for c in self.constraints])
