from grammar_utils._internal import LR1Parser  # noqa
from grammar_utils.grammars import load_grammar_and_lexer


def load_lr1_parser(name: str) -> tuple[str, str]:
    """

    Load a LR(1) parser for the given name.
    Currently supported:
    - json
    - sparql

    """
    return LR1Parser.from_grammar_and_lexer(*load_grammar_and_lexer(name))
