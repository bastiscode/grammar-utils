use std::fs::{self, read_to_string};
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, Criterion};
use grammar_utils::LR1GrammarConstraint;
use grammar_utils::{
    Constraint, ExactLR1GrammarConstraint, LR1GrammarParser, RegularExpressionConstraint,
};

fn load_continuations() -> Vec<Vec<u8>> {
    let dir = env!("CARGO_MANIFEST_DIR");
    let continuations_json = fs::read(PathBuf::from(dir).join("resources/test/continuations.json"))
        .expect("failed to read file");
    // use serde to deserialize continuations array from json
    serde_json::from_slice::<Vec<String>>(&continuations_json)
        .unwrap()
        .into_iter()
        .map(|c| c.as_bytes().to_vec())
        .collect()
}

fn load_grammars() -> impl Iterator<Item = (String, PathBuf, PathBuf, Vec<(String, Vec<u8>)>)> {
    let dir = env!("CARGO_MANIFEST_DIR");
    fs::read_dir(PathBuf::from(dir).join("grammars"))
        .unwrap()
        .map(move |entry| {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            let y = entry.path().join(format!("{}.y", name));
            let l = entry.path().join(format!("{}.l", name));
            // load examples from dir/granmars/<name>/examples/*.txt
            let examples = fs::read_dir(entry.path().join("examples"))
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    // get the file name without the extension
                    let name = entry.path().file_stem().unwrap().to_str().unwrap().into();
                    let example = read_to_string(entry.path())
                        .unwrap()
                        .trim_end_matches(&['\r', '\n'])
                        .as_bytes()
                        .to_vec();
                    (name, example)
                })
                .collect();
            (name, y, l, examples)
        })
}

fn bench_re_constraint(c: &mut Criterion) {
    let conts = load_continuations();

    let re = RegularExpressionConstraint::new(r"yes|no|maybe", conts.clone()).unwrap();
    let state = re.get_state(b"may").unwrap();
    c.bench_function("re_mc_get_valid_continuations", |b| {
        b.iter(|| re.get_valid_continuations(&state))
    });

    let re = RegularExpressionConstraint::new(r"\w+@\w+\.(com|de|org)", conts.clone()).unwrap();
    let state = re.get_state(b"test").unwrap();
    c.bench_function("re_email1_get_valid_continuations", |b| {
        b.iter(|| re.get_valid_continuations(&state))
    });
    let state = re.get_state(b"test@gmai").unwrap();
    c.bench_function("re_email2_get_valid_continuations", |b| {
        b.iter(|| re.get_valid_continuations(&state))
    });
    let state = re.get_state(b"test@gmail.c").unwrap();
    c.bench_function("re_email3_get_valid_continuations", |b| {
        b.iter(|| re.get_valid_continuations(&state))
    });

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test/re-examples");
    let files = ["json.txt", "template.txt", "triples.txt"];
    let prefixes = [
        r#"{
    "name": "irableirny",
    "age": "60",
    "email": "strvir"#,
        r#"<name>irableirny</name>
<age>60</age>
<email>strvir"#,
        r#"<bos>obiernobpb</eos> <bop>aseimarbar</eop> <boo>positorybo<eoo> .
<bos>abilushcji</eos> <bop>nomek"#,
    ];
    for (file, prefix) in files.iter().zip(prefixes) {
        let path = dir.join(file);
        let file_name = path.file_stem().unwrap().to_str().unwrap();
        let re = RegularExpressionConstraint::from_file(&path, conts.clone()).unwrap();
        let state = re.get_state(prefix.as_bytes()).unwrap();
        assert!(
            !re.get_valid_continuations(&state).is_empty(),
            "'{prefix}' has no valid continuations"
        );
        c.bench_function(
            &format!("re_file_{file_name}_get_valid_continuations"),
            |b| b.iter(|| re.get_valid_continuations(&state)),
        );
    }
}

fn bench_lr1_constraint(c: &mut Criterion) {
    let conts = load_continuations();
    for (name, grammar, tokens, examples) in load_grammars() {
        let exact_lr_constraint =
            ExactLR1GrammarConstraint::from_files(&grammar, &tokens, conts.clone()).unwrap();
        let lr_constraint =
            LR1GrammarConstraint::from_files(grammar, tokens, conts.clone()).unwrap();

        let state = exact_lr_constraint.get_start_state();
        let conts = exact_lr_constraint.get_valid_continuations(&state);
        c.bench_function(
            &format!("exact_lr1_{name}_empty_get_valid_continuations"),
            |b| b.iter(|| exact_lr_constraint.get_valid_continuations(&state)),
        );
        c.bench_function(&format!("exact_lr1_{name}_empty_get_next_state"), |b| {
            b.iter(|| exact_lr_constraint.get_next_state(&state, conts[0]))
        });
        let state = lr_constraint.get_start_state();
        let conts = lr_constraint.get_valid_continuations(&state);
        c.bench_function(
            &format!("standard_lr1_{name}_empty_get_valid_continuations"),
            |b| b.iter(|| lr_constraint.get_valid_continuations(&state)),
        );
        c.bench_function(&format!("standard_lr1_{name}_empty_get_next_state"), |b| {
            b.iter(|| lr_constraint.get_next_state(&state, conts[0]))
        });
        for (ex_name, example) in examples {
            let state = exact_lr_constraint.get_state(&example).unwrap();
            let conts = exact_lr_constraint.get_valid_continuations(&state);
            println!(
                "testing {} {}:\n{}",
                name,
                ex_name,
                String::from_utf8_lossy(&example),
            );
            println!("{} exact continuations", conts.len());
            c.bench_function(
                &format!("exact_lr1_{name}_{ex_name}_get_valid_continuations"),
                |b| b.iter(|| exact_lr_constraint.get_valid_continuations(&state)),
            );
            c.bench_function(&format!("exact_lr1_{name}_{ex_name}_get_next_state"), |b| {
                b.iter(|| exact_lr_constraint.get_next_state(&state, conts[0]))
            });
            let state = lr_constraint.get_state(&example).unwrap();
            let conts = lr_constraint.get_valid_continuations(&state);
            println!("{} standard continuations", conts.len());
            c.bench_function(
                &format!("standard_lr1_{name}_{ex_name}_get_valid_continuations"),
                |b| b.iter(|| lr_constraint.get_valid_continuations(&state)),
            );
            c.bench_function(
                &format!("standard_lr1_{name}_{ex_name}_get_next_state"),
                |b| b.iter(|| lr_constraint.get_next_state(&state, conts[0])),
            );
        }
    }
}

fn bench_lr1_parser(c: &mut Criterion) {
    for (name, grammar, tokens, examples) in load_grammars() {
        let parser = LR1GrammarParser::from_files(grammar, tokens).unwrap();
        for (ex_name, example) in examples {
            c.bench_function(&format!("lr1_prefix_parse_{name}_{ex_name}"), |b| {
                b.iter(|| parser.prefix_parse(&example, false, false).unwrap())
            });
        }
    }
}

criterion_group!(
    benches,
    bench_re_constraint,
    bench_lr1_constraint,
    bench_lr1_parser
);
criterion_main!(benches);
