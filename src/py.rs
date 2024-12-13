use std::{
    num::NonZeroUsize,
    sync::{mpsc::channel, Arc, Mutex},
};

use anyhow::anyhow;
use lru::LruCache;
use numpy::{ndarray::Array1, IntoPyArray, PyArray1};
use pyo3::{
    prelude::*,
    types::{PyDict, PyList},
};
use rayon::spawn_fifo;
use regex_automata::util::primitives::StateID;

use crate::{
    Constraint, ExactLR1GrammarConstraint, LR1GrammarConstraint, LR1GrammarParser, LR1Parse,
    LR1State, RegularExpressionConstraint, TokenAndSpan,
};

#[derive(Clone)]
struct RegexInner {
    state: StateID,
    indices: Array1<i32>,
    is_match: bool,
    is_invalid: bool,
}

#[pyclass]
struct RegexConstraint {
    constraint: Arc<RegularExpressionConstraint>,
    inner: Arc<Mutex<RegexInner>>,
}

impl RegexConstraint {
    fn init(constraint: RegularExpressionConstraint) -> Self {
        let state = constraint.get_start_state();
        let indices = constraint
            .get_valid_continuations(&state)
            .into_iter()
            .map(|v| v as i32)
            .collect();
        let is_match = constraint.is_match_state(&state);
        Self {
            constraint: Arc::new(constraint),
            inner: Arc::new(Mutex::new(RegexInner {
                state,
                indices,
                is_match,
                is_invalid: false,
            })),
        }
    }
}

#[pymethods]
impl RegexConstraint {
    #[new]
    fn new(regex: &str, continuations: Vec<Vec<u8>>) -> anyhow::Result<Self> {
        RegularExpressionConstraint::new(regex, continuations)
            .map(Self::init)
            .map_err(|e| {
                anyhow!(
                    "failed to create regular expression constraint from regex '{}': {}",
                    regex,
                    e
                )
            })
    }

    #[staticmethod]
    fn from_file(path: &str, continuations: Vec<Vec<u8>>) -> anyhow::Result<Self> {
        RegularExpressionConstraint::from_file(path, continuations)
            .map(Self::init)
            .map_err(|e| {
                anyhow!(
                    "failed to create regular expression constraint from file '{}': {}",
                    path,
                    e
                )
            })
    }

    #[pyo3(signature = (prefix = None))]
    fn reset(&self, prefix: Option<Vec<u8>>) -> anyhow::Result<()> {
        let Some(state) = self.constraint.get_state(&prefix.unwrap_or_default()) else {
            return Err(anyhow!("failed to reset to given prefix"));
        };
        self.inner
            .lock()
            .map(|mut inner| {
                inner.state = state;
                inner.indices = self
                    .constraint
                    .get_valid_continuations(&inner.state)
                    .into_iter()
                    .map(|v| v as i32)
                    .collect();
                inner.is_match = self.constraint.is_match_state(&inner.state);
                inner.is_invalid = false;
            })
            .map_err(|_| anyhow!("error locking inner state"))
    }

    fn clone(&self) -> anyhow::Result<Self> {
        self.inner
            .lock()
            .map(|inner| Self {
                constraint: self.constraint.clone(),
                inner: Arc::new(Mutex::new(inner.clone())),
            })
            .map_err(|_| anyhow!("error locking inner state"))
    }

    fn get<'py>(&self, py: Python<'py>) -> anyhow::Result<Bound<'py, PyArray1<i32>>> {
        self.inner
            .lock()
            .map(|inner| inner.indices.clone().into_pyarray(py))
            .map_err(|_| anyhow!("error locking inner state"))
    }

    fn is_invalid(&self) -> anyhow::Result<bool> {
        self.inner
            .lock()
            .map(|inner| inner.is_invalid || (inner.indices.is_empty() && !inner.is_match))
            .map_err(|_| anyhow!("error locking inner state"))
    }

    fn is_match(&self) -> anyhow::Result<bool> {
        self.inner
            .lock()
            .map(|inner| inner.is_match)
            .map_err(|_| anyhow!("error locking inner state"))
    }

    fn next(&self, index: usize) -> anyhow::Result<()> {
        let inner = self.inner.clone();
        let constraint = self.constraint.clone();
        let (tx, rx) = channel();
        spawn_fifo(move || {
            let mut inner = inner.lock().expect("error locking inner state");
            tx.send(()).expect("failed to send on channel");
            let Some(next_state) = constraint.get_next_state(&inner.state, index) else {
                inner.is_invalid = true;
                return;
            };
            inner.state = next_state;
            inner.indices = constraint
                .get_valid_continuations(&inner.state)
                .into_iter()
                .map(|v| v as i32)
                .collect();
            inner.is_match = constraint.is_match_state(&inner.state);
        });
        // wait until spawned thread signals that is has locked
        // the inner state, otherwise some unexpected behavior could occurr
        rx.recv()?;
        Ok(())
    }
}

enum LR1Type {
    Exact(ExactLR1GrammarConstraint),
    Regular(LR1GrammarConstraint),
}

#[derive(Clone)]
struct LR1Inner {
    state: LR1State,
    indices: Array1<i32>,
    is_match: bool,
    is_invalid: bool,
}

type LR1ConstraintCache = LruCache<LR1State, (Array1<i32>, bool)>;

#[pyclass]
struct LR1Constraint {
    constraint: Arc<LR1Type>,
    inner: Arc<Mutex<LR1Inner>>,
    cache: Arc<Mutex<LR1ConstraintCache>>,
}

impl LR1Type {
    fn get_state(&self, prefix: &[u8]) -> Option<LR1State> {
        match self {
            LR1Type::Exact(inner) => inner.get_state(prefix),
            LR1Type::Regular(inner) => inner.get_state(prefix),
        }
    }

    fn get_start_state(&self) -> LR1State {
        match self {
            LR1Type::Exact(inner) => inner.get_start_state(),
            LR1Type::Regular(inner) => inner.get_start_state(),
        }
    }

    fn get_valid_continuations(&self, state: &LR1State) -> Array1<i32> {
        match self {
            LR1Type::Exact(inner) => inner.get_valid_continuations(state),
            LR1Type::Regular(inner) => inner.get_valid_continuations(state),
        }
        .into_iter()
        .map(|v| v as i32)
        .collect()
    }

    fn get_next_state(&self, state: &LR1State, continuation: usize) -> Option<LR1State> {
        match self {
            LR1Type::Exact(inner) => inner.get_next_state(state, continuation),
            LR1Type::Regular(inner) => inner.get_next_state(state, continuation),
        }
    }

    fn is_match_state(&self, state: &LR1State) -> bool {
        match self {
            LR1Type::Exact(inner) => inner.is_match_state(state),
            LR1Type::Regular(inner) => inner.is_match_state(state),
        }
    }

    fn only_skippable_matching(&self, state: &LR1State) -> bool {
        match self {
            LR1Type::Exact(inner) => inner.only_skippable_matching(state),
            LR1Type::Regular(inner) => inner.only_skippable_matching(state),
        }
    }
}

impl LR1Constraint {
    fn init(constraint: LR1Type, lru_cache_size: Option<usize>) -> Self {
        let state = constraint.get_start_state();
        let indices = constraint.get_valid_continuations(&state);
        let is_match = constraint.is_match_state(&state);
        // get cache size from env variable TEXT_UTILS_LR1_CACHE_SIZE
        let cache_size = lru_cache_size
            .and_then(NonZeroUsize::new)
            .unwrap_or(NonZeroUsize::new(8192).unwrap());
        let mut cache = LruCache::new(cache_size);
        cache.put(state.clone(), (indices.clone(), is_match));
        Self {
            constraint: Arc::new(constraint),
            inner: Arc::new(Mutex::new(LR1Inner {
                state,
                indices,
                is_match,
                is_invalid: false,
            })),
            cache: Arc::new(Mutex::new(cache)),
        }
    }
}

#[pymethods]
impl LR1Constraint {
    #[new]
    #[pyo3(signature = (grammar, lexer, continuations, exact=false, lru_cache_size=None))]
    fn new(
        grammar: &str,
        lexer: &str,
        continuations: Vec<Vec<u8>>,
        exact: bool,
        lru_cache_size: Option<usize>,
    ) -> anyhow::Result<Self> {
        let constraint = if exact {
            LR1Type::Exact(
                ExactLR1GrammarConstraint::new(grammar, lexer, continuations)
                    .map_err(|e| anyhow!("failed to create LR(1) grammar constraint: {}", e))?,
            )
        } else {
            LR1Type::Regular(
                LR1GrammarConstraint::new(grammar, lexer, continuations)
                    .map_err(|e| anyhow!("failed to create LR(1) grammar constraint: {}", e))?,
            )
        };
        Ok(Self::init(constraint, lru_cache_size))
    }

    #[staticmethod]
    #[pyo3(signature = (grammar_path, lexer_path, continuations, exact=false, lru_cache_size=None))]
    fn from_files(
        grammar_path: &str,
        lexer_path: &str,
        continuations: Vec<Vec<u8>>,
        exact: bool,
        lru_cache_size: Option<usize>,
    ) -> anyhow::Result<Self> {
        let constraint = if exact {
            LR1Type::Exact(
                ExactLR1GrammarConstraint::from_files(grammar_path, lexer_path, continuations)
                    .map_err(|e| anyhow!("failed to create LR(1) grammar constraint: {}", e))?,
            )
        } else {
            LR1Type::Regular(
                LR1GrammarConstraint::from_files(grammar_path, lexer_path, continuations)
                    .map_err(|e| anyhow!("failed to create LR(1) grammar constraint: {}", e))?,
            )
        };
        Ok(Self::init(constraint, lru_cache_size))
    }

    #[pyo3(signature = (prefix = None))]
    fn reset(&self, prefix: Option<Vec<u8>>) -> anyhow::Result<()> {
        let Some(state) = self.constraint.get_state(&prefix.unwrap_or_default()) else {
            return Err(anyhow!("failed to reset to given prefix"));
        };
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| anyhow!("error locking inner state"))?;
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| anyhow!("error locking cache"))?;

        inner.state = state;
        inner.is_invalid = false;
        if let Some((indices, is_match)) = cache.get(&inner.state).cloned() {
            inner.indices = indices;
            inner.is_match = is_match;
        } else {
            inner.indices = self.constraint.get_valid_continuations(&inner.state);
            inner.is_match = self.constraint.is_match_state(&inner.state);
            cache.put(inner.state.clone(), (inner.indices.clone(), inner.is_match));
        }
        Ok(())
    }

    fn clone(&self) -> anyhow::Result<Self> {
        self.inner
            .lock()
            .map(|inner| Self {
                constraint: self.constraint.clone(),
                inner: Arc::new(Mutex::new(inner.clone())),
                cache: self.cache.clone(),
            })
            .map_err(|_| anyhow!("error locking inner state"))
    }

    fn get<'py>(&self, py: Python<'py>) -> anyhow::Result<Bound<'py, PyArray1<i32>>> {
        self.inner
            .lock()
            .map(|inner| {
                if inner.is_match && self.constraint.only_skippable_matching(&inner.state) {
                    // should stop, return empty indices
                    vec![].into()
                } else {
                    inner.indices.clone()
                }
                .into_pyarray(py)
            })
            .map_err(|_| anyhow!("error locking inner state"))
    }

    fn is_invalid(&self) -> anyhow::Result<bool> {
        self.inner
            .lock()
            .map(|inner| inner.is_invalid || (inner.indices.is_empty() && !inner.is_match))
            .map_err(|_| anyhow!("error locking inner state"))
    }

    fn is_match(&self) -> anyhow::Result<bool> {
        self.inner
            .lock()
            .map(|inner| inner.is_match)
            .map_err(|_| anyhow!("error locking inner state"))
    }

    fn next(&self, index: usize) -> anyhow::Result<()> {
        let inner = self.inner.clone();
        let constraint = self.constraint.clone();
        let cache = self.cache.clone();
        let (tx, rx) = channel();
        spawn_fifo(move || {
            let mut inner = inner.lock().expect("error locking inner state");
            let mut cache = cache.lock().expect("error locking cache");
            tx.send(()).expect("failed to send on channel");
            let Some(next_state) = constraint.get_next_state(&inner.state, index) else {
                inner.is_invalid = true;
                return;
            };
            inner.state = next_state;
            if let Some((indices, is_match)) = cache.get(&inner.state).cloned() {
                inner.indices = indices;
                inner.is_match = is_match;
            } else {
                inner.indices = constraint.get_valid_continuations(&inner.state);
                inner.is_match = constraint.is_match_state(&inner.state);
                cache.put(inner.state.clone(), (inner.indices.clone(), inner.is_match));
            }
        });
        // wait until spawned thread signals that is has locked
        // the inner state, otherwise some unexpected behavior could occurr
        rx.recv()?;
        Ok(())
    }
}

#[pyclass]
pub struct LR1Parser {
    inner: LR1GrammarParser,
}

#[pymethods]
impl LR1Parser {
    #[new]
    fn new(grammar: &str, lexer: &str) -> anyhow::Result<Self> {
        let inner = LR1GrammarParser::new(grammar, lexer).map_err(|e| {
            anyhow!(
                "failed to create LR(1) grammar parser from grammar {} and lexer {}: {}",
                grammar,
                lexer,
                e
            )
        })?;
        Ok(Self { inner })
    }

    #[staticmethod]
    fn from_files(grammar_path: &str, lexer_path: &str) -> anyhow::Result<Self> {
        let inner = LR1GrammarParser::from_files(grammar_path, lexer_path).map_err(|e| {
            anyhow!(
                "failed to create LR(1) grammar parser from files {} and {}: {}",
                grammar_path,
                lexer_path,
                e
            )
        })?;
        Ok(Self { inner })
    }

    #[pyo3(signature = (input, skip_empty = false, collapse_single = false))]
    fn prefix_parse<'py>(
        &self,
        py: Python<'py>,
        input: &[u8],
        skip_empty: bool,
        collapse_single: bool,
    ) -> anyhow::Result<(Bound<'py, PyDict>, Vec<u8>)> {
        let (parse, end) = self
            .inner
            .prefix_parse(input, skip_empty, collapse_single)
            .map_err(|e| anyhow!("failed to parse input: {e}"))?;
        let parse_dict = parse_into_py(std::str::from_utf8(input)?, &parse, py)?;
        Ok((parse_dict, end.to_vec()))
    }

    #[pyo3(signature = (input, skip_empty = false, collapse_single = false))]
    fn parse<'py>(
        &self,
        py: Python<'py>,
        input: &str,
        skip_empty: bool,
        collapse_single: bool,
    ) -> anyhow::Result<Bound<'py, PyDict>> {
        let parse = self
            .inner
            .parse(input, skip_empty, collapse_single)
            .map_err(|e| anyhow!("failed to parse input: {e}"))?;
        Ok(parse_into_py(input, &parse, py)?)
    }

    fn lex(&self, input: &str) -> anyhow::Result<Vec<TokenAndSpan>> {
        self.inner
            .lex(input)
            .map_err(|e| anyhow!("failed to lex input: {e}"))
    }
}

fn parse_into_py<'py>(
    text: impl AsRef<[u8]>,
    parse: &LR1Parse<'_>,
    py: Python<'py>,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    let bytes = text.as_ref();
    match parse {
        LR1Parse::Empty(name) => {
            dict.set_item("name", name)?;
        }
        LR1Parse::Terminal(name, span, value) => {
            dict.set_item("name", name)?;
            let &(start, end) = span;
            dict.set_item("value", String::from_utf8_lossy(value))?;
            dict.set_item("byte_span", (start, end))?;
        }
        LR1Parse::NonTerminal(name, children) => {
            dict.set_item("name", name)?;
            let children = PyList::new(
                py,
                children
                    .iter()
                    .map(|c| parse_into_py(bytes, c, py))
                    .collect::<PyResult<Vec<_>>>()?,
            )?;
            dict.set_item("children", children)?;
        }
    };
    Ok(dict)
}

/// The module containing all python bindings for the grammar utils library.
#[pymodule]
fn _internal(_: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RegexConstraint>()?;
    m.add_class::<LR1Constraint>()?;
    m.add_class::<LR1Parser>()?;
    Ok(())
}
