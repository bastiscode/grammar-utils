from importlib import resources


def load_grammar_and_lexer(name: str) -> tuple[str, str]:
    """

    Read the grammar and lexer definitions for the given name.
    Currently supported:
    - json
    - sparql

    """
    grammar = resources.read_text(f"grammar_utils.grammars.{name}", f"{name}.y")
    lexer = resources.read_text(f"grammar_utils.grammars.{name}", f"{name}.l")
    return grammar, lexer
