"""Type stubs for grammar_utils._internal module."""

from typing import Any, final

import numpy as np
import numpy.typing as npt

@final
class RegexConstraint:
    """Constraint based on a regular expression."""

    def __init__(self, regex: str, continuations: list[list[int]]) -> None:
        """
        Create a regex constraint.

        Args:
            regex: Regular expression pattern
            continuations: List of byte continuations (vocabulary)
        """
        ...

    @staticmethod
    def from_file(path: str, continuations: list[list[int]]) -> RegexConstraint:
        """
        Create a regex constraint from a file.

        Args:
            path: Path to a file containing the regex pattern
            continuations: List of byte continuations (vocabulary)

        Returns:
            RegexConstraint instance
        """
        ...

    def reset(self, prefix: bytes | None = None) -> None:
        """
        Reset the constraint to the initial state, optionally with a prefix.

        Args:
            prefix: Optional byte prefix to reset to
        """
        ...

    def clone(self) -> RegexConstraint:
        """
        Create a copy of the constraint with the current state.

        Returns:
            Cloned RegexConstraint instance
        """
        ...

    def get(self) -> npt.NDArray[np.int32]:
        """
        Get the valid continuation indices for the current state.

        Returns:
            Array of valid continuation indices
        """
        ...

    def is_invalid(self) -> bool:
        """
        Check if the current state is invalid.

        Returns:
            True if the state is invalid
        """
        ...

    def is_match(self) -> bool:
        """
        Check if the current state is a match state.

        Returns:
            True if the state is a match
        """
        ...

    def next(self, index: int) -> None:
        """
        Advance the state by the chosen continuation index.

        Args:
            index: Continuation index to advance by
        """
        ...

@final
class LR1Constraint:
    """Constraint based on an LR(1) grammar."""

    def __init__(
        self,
        grammar: str,
        lexer: str,
        continuations: list[list[int]],
        exact: bool = False,
        lru_cache_size: int | None = None,
    ) -> None:
        """
        Create an LR(1) grammar constraint.

        Args:
            grammar: Grammar definition string
            lexer: Lexer definition string
            continuations: List of byte continuations (vocabulary)
            exact: Use exact constraint matching (default: False)
            lru_cache_size: Size of the LRU cache (default: 8192)
        """
        ...

    @staticmethod
    def from_files(
        grammar_path: str,
        lexer_path: str,
        continuations: list[list[int]],
        exact: bool = False,
        lru_cache_size: int | None = None,
    ) -> LR1Constraint:
        """
        Create an LR(1) grammar constraint from files.

        Args:
            grammar_path: Path to the grammar file
            lexer_path: Path to the lexer file
            continuations: List of byte continuations (vocabulary)
            exact: Use exact constraint matching (default: False)
            lru_cache_size: Size of the LRU cache (default: 8192)

        Returns:
            LR1Constraint instance
        """
        ...

    def reset(self, prefix: bytes | None = None) -> None:
        """
        Reset the constraint to the initial state, optionally with a prefix.

        Args:
            prefix: Optional byte prefix to reset to
        """
        ...

    def clone(self) -> LR1Constraint:
        """
        Create a copy of the constraint with the current state.

        Returns:
            Cloned LR1Constraint instance
        """
        ...

    def get(self) -> npt.NDArray[np.int32]:
        """
        Get the valid continuation indices for the current state.

        Returns:
            Array of valid continuation indices
        """
        ...

    def is_invalid(self) -> bool:
        """
        Check if the current state is invalid.

        Returns:
            True if the state is invalid
        """
        ...

    def is_match(self) -> bool:
        """
        Check if the current state is a match state.

        Returns:
            True if the state is a match
        """
        ...

    def next(self, index: int) -> None:
        """
        Advance the state by the chosen continuation index.

        Args:
            index: Continuation index to advance by
        """
        ...

@final
class LR1Parser:
    """LR(1) grammar parser."""

    def __init__(self, grammar: str, lexer: str) -> None:
        """
        Create an LR(1) parser.

        Args:
            grammar: Grammar definition string
            lexer: Lexer definition string
        """
        ...

    @staticmethod
    def from_files(grammar_path: str, lexer_path: str) -> LR1Parser:
        """
        Create an LR(1) parser from files.

        Args:
            grammar_path: Path to the grammar file
            lexer_path: Path to the lexer file

        Returns:
            LR1Parser instance
        """
        ...

    def prefix_parse(
        self,
        input: bytes,
        skip_empty: bool = False,
        collapse_single: bool = False,
    ) -> tuple[dict[str, Any], bytes]:
        """
        Parse a byte prefix, returning the parse tree and remaining bytes.

        Args:
            input: Input bytes to parse
            skip_empty: Skip empty nodes in the parse tree (default: False)
            collapse_single: Collapse single-child nodes (default: False)

        Returns:
            Tuple of (parse tree dict, remaining unparsed bytes)
        """
        ...

    def parse(
        self,
        input: str,
        skip_empty: bool = False,
        collapse_single: bool = False,
    ) -> dict[str, Any]:
        """
        Parse a complete input string.

        Args:
            input: Input string to parse
            skip_empty: Skip empty nodes in the parse tree (default: False)
            collapse_single: Collapse single-child nodes (default: False)

        Returns:
            Parse tree as a dict
        """
        ...

    def lex(self, input: str) -> list[tuple[str | None, tuple[int, int]]]:
        """
        Lex an input string into tokens.

        Args:
            input: Input string to lex

        Returns:
            List of (token_name, (start, end)) tuples
        """
        ...

__all__ = [
    "LR1Constraint",
    "LR1Parser",
    "RegexConstraint",
]
