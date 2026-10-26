use super::walk::{Value, object};
use super::*;

/// Real save bodies (read-only): `PYLUNE_SAVES=<save folder>`. Reports how
/// many top-level objects decode exactly.
#[test]
#[ignore = "reads the local save files"]
fn real_bodies_decode_exactly() {
    let Ok(dir) = std::env::var("PYLUNE_SAVES") else {
        return;
    };
    let files = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| std::fs::read(e.path()).is_ok_and(|b| b.starts_with(b"SAVE")));
    files.for_each(|e| {
        let save = crate::savefile::decode(&std::fs::read(e.path()).unwrap()).unwrap();
        let body = parse(&save.body).unwrap();
        let results: Vec<_> = (0..body.toc.len())
            .map(|i| object(&body, &save.body, i))
            .collect();
        let ok = results.iter().filter(|r| r.is_ok()).count();
        let mut failed: Vec<String> = results
            .iter()
            .zip(&body.toc)
            .filter_map(|(r, t)| {
                r.as_ref()
                    .err()
                    .map(|err| format!("{}: {err}", body.types[t.class as usize].name))
            })
            .collect();
        failed.sort();
        failed.dedup();
        println!(
            "{}: {} types, {ok}/{} objects exact; failures: {:?}",
            e.path().display(),
            body.types.len(),
            body.toc.len(),
            &failed[..failed.len().min(6)]
        );
        let _ = Value::Scalar(Vec::new());
    });
}

/// Prints the top-level types of one save with counts and field names.
#[test]
#[ignore = "reads the local save files"]
fn survey_types() {
    let Ok(path) = std::env::var("PYLUNE_SAVE") else {
        return;
    };
    let save = crate::savefile::decode(&std::fs::read(path).unwrap()).unwrap();
    let body = parse(&save.body).unwrap();
    let mut counts = std::collections::BTreeMap::new();
    body.toc
        .iter()
        .for_each(|t| *counts.entry(t.class).or_insert(0) += 1);
    counts.iter().for_each(|(class, n)| {
        let t = &body.types[*class as usize];
        let fields: Vec<String> = t
            .fields
            .iter()
            .map(|f| format!("{}:{}", f.name, f.type_name))
            .collect();
        println!("{n:>4} x {} [{}]", t.name, fields.join(", "));
    });
}

/// Prints the tree of a save's first objects (read-only exploration).
#[test]
#[ignore = "reads the local save files"]
fn print_tree() {
    let Ok(path) = std::env::var("PYLUNE_SAVE") else {
        return;
    };
    let save = crate::savefile::decode(&std::fs::read(path).unwrap()).unwrap();
    let body = parse(&save.body).unwrap();
    fn show(nodes: &[crate::savefile::view::Node], depth: usize, budget: &mut usize) {
        for n in nodes {
            if *budget == 0 {
                return;
            }
            *budget -= 1;
            println!("{}{}", "  ".repeat(depth), n.label);
            show(&n.children, depth + 1, budget);
        }
    }
    let wanted: Vec<String> = std::env::var("PYLUNE_TYPES")
        .map(|t| t.split(',').map(str::to_string).collect())
        .unwrap_or_default();
    (0..body.toc.len()).for_each(|i| {
        let name = &body.types[body.toc[i].class as usize].name;
        if !wanted.is_empty() && !wanted.contains(name) {
            return;
        }
        let object = object(&body, &save.body, i).unwrap();
        println!("== {name}");
        show(&crate::savefile::view::tree(&body, &object), 1, &mut 40);
    });
}

#[test]
fn hostile_nesting_is_an_error_not_a_crash() {
    let body = Body {
        types: vec![TypeDef {
            name: "T".into(),
            fields: vec![FieldDef {
                name: "_c".into(),
                type_name: "T".into(),
                kind: 4,
                size: 8,
                aux: 0,
            }],
        }],
        toc: Vec::new(),
    };
    // Top-level header, then 1000 embedded objects each containing the next.
    let mut data = vec![1, 0, 1, 0, 0, 0, 0];
    (0..1000).for_each(|_| {
        data.extend_from_slice(&[1, 0, 1, 0, 0]);
        data.extend_from_slice(&[0; 13]);
        data.extend_from_slice(&[0; 4]);
    });
    let body = Body {
        toc: vec![TocEntry {
            class: 0,
            offset: 0,
            size: data.len() as u32,
        }],
        ..body
    };
    let err = object(&body, &data, 0).unwrap_err().to_string();
    assert!(err.contains("nested too deeply"), "{err}");
}
