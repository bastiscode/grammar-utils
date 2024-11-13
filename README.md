## Grammar utilities

This repository contains Python utilities (backed by Rust) to parse and constrain text with regular expressions
and context-free grammars (LR(1)). Parsing is supported both for prefixes and full strings.

Context-free [grammars](grammars) already included in this repository are:
- JSON
- SPARQL

### Installation

You can install the Python package from PyPI:

```bash
pip install grammar-utils
```

Windows (x64) and Linux are currently supported when installing from PyPI.

Alternatively, you can clone this repository and build the package yourself:

```bash
git clone https://github.com/bastiscode/grammar-utils
cd grammar-utils
pip install maturin[patchelf]
maturin develop --release
```

### Usage

Two use cases are supported by this library: parsing and constraining.

#### Parsing

Given a context-free grammar, parse a string and return the corresponding parse tree.

```python
from grammar_utils.parse import load_lr1_parser

parser = load_lr1_parser("json")
tree = parser.parse('{"key": "value"}')
print(tree)
# you can calso get a pruned parse tree, skipping empty or collapsing single child nodes
pruned_tree = parser.parse('{"key": "value"}', skip_empty=True, collapse_single=True)
print(pruned_tree)
```

Parsing is also supported for prefixes, in which case the input should be a list of bytes
and not a string. Here a tree for the already fixed terminals is returned, as well as the
suffix of the input where we do not know yet what the next terminal is.

```python
from grammar_utils.parse import load_lr1_parser

parser = load_lr1_parser("json")
tree, rest = parser.prefix_parse(b'{"key"")
print(tree)
print(rest)
# pruning is also supported here
pruned_tree, rest = parser.prefix_parse(b'{"key"', skip_empty=True, collapse_single=True)
print(pruned_tree)
print(rest)
```

You can also use your own grammars.

```python
from grammar_utils.parse import LR1Parser

# define your own grammar and lexer
grammar = "..."
lexer = "..."
parser = LR1Parser.from_grammar_and_lexer(grammar, lexer, vocab)
```

#### Constraining

Constraints are used to check what symbols from the vocabulary can follow the current prefix
such that the regular expression or context-free grammar can still be satisfied.

```python
import random
from grammar_utils import load_byte_vocab
from grammar_utils.constrain import load_lr1_constraint, load_regex_constraint

vocab = load_byte_vocab()
constraint = load_lr1_constraint("json", vocab)
# reset constraint to a given prefix, default is an empty prefix
constraint.reset(b'{"key"')
# get the next possible symbols
next_indices = constraint.get()
# the indices refer to the vocabulary (decode only for human-readable strings)
print(f"allowed continuations: {[bytes(vocab[i]).decode() for i in next_indices]}")
# you can forward the constraint with a valid index
constraint.next(random.choice(next_indices))
# check if constraint is satisfied (should be False)
print(constraint.is_match())

# same for regular expressions
constraint = load_regex_constraint("boolean", vocab)
constraint.reset(b"tr")
next_indices = constraint.get()
# should only be 'u'
print(f"allowed continuations: {[bytes(vocab[i]).decode() for i in next_indices]}")
constraint.next(next_indices[0])
print(constraint.is_match())
next_indices = constraint.get()
# should only be 'e'
print(f"allowed continuations: {[bytes(vocab[i]).decode() for i in next_indices]}")
constraint.next(next_indices[0])
# should be True
print(constraint.is_match())
```

You can also use your own grammars and regexes.

```python
from grammar_utils import load_byte_vocab
from grammar_utils.constrain import LR1Constraint, RegexConstraint

vocab = load_byte_vocab()

# define your own grammar and lexer
grammar = "..."
lexer = "..."
constraint = LR1Constraint.from_grammar_and_lexer(grammar, lexer, vocab)

# define your own regex
regex = "..."
constraint = RegexConstraint.from_regex(regex, vocab)
```

### Use cases

#### Forcing a language model to generate structured text

The following example shows how to use a regex constraint to force GPT2
to output either "true" or "false" after a given prompt.

```python
import torch
from transformers import AutoTokenizer, AutoModelForCausalLM
from grammar_utils.constrain import load_regex_constraint

gpt2 = AutoModelForCausalLM.from_pretrained("gpt2")
tokenizer = AutoTokenizer.from_pretrained("gpt2")
vocab = [
    token.replace("Ġ", " ").encode()
    for token, _ in sorted(tokenizer.get_vocab().items(), key=lambda x: x[1])
]
constraint = load_regex_constraint("boolean", vocab)
prefix = "Constrained decoding is cool: "
input_ids = tokenizer.encode(prefix)
while not (constraint.is_match() or constraint.is_invalid()):
    input_tensor = torch.tensor([input_ids])
    logits = gpt2(input_tensor).logits
    valid_indices = torch.from_numpy(constraint.get())
    valid_logits = logits[0, -1, valid_indices]
    index = valid_indices[torch.argmax(valid_logits)]
    constraint.next(index)
    input_ids.append(index)
    print(tokenizer.decode(input_ids))
```
