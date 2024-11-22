import argparse
from pprint import pprint

from grammar_utils._internal import LR1Parser  # noqa
from grammar_utils.grammars import load_grammar_and_lexer


def load_lr1_parser(name: str) -> tuple[str, str]:
    """

    Load a LR(1) parser for the given name.
    Currently supported:
    - json
    - sparql

    """
    return LR1Parser(*load_grammar_and_lexer(name))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("grammar", type=str, help="The grammar to load")
    parser.add_argument("lexer", type=str, help="The lexer to load")
    parser.add_argument("input", type=str, help="The input to parse")
    parser.add_argument(
        "-p", "--prefix", action="store_true", help="Use prefix parsing"
    )
    args = parser.parse_args()
    parser = LR1Parser.from_files(args.grammar, args.lexer)
    if args.prefix:
        tree, rest = parser.prefix_parse(args.input.encode())
        pprint(tree)
        pprint(rest)
    else:
        tree = parser.parse(args.input)
        pprint(tree)
