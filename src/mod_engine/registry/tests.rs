use super::*;

fn reg(names: &[&str]) -> Registry {
    Registry {
        mods: names.iter().map(|n| discovered(n, 0)).collect(),
    }
}

fn order(r: &Registry) -> Vec<&str> {
    r.mods.iter().map(|m| m.name.as_str()).collect()
}

#[test]
fn moves_and_flags() {
    let r = reg(&["a", "b", "c"]);
    assert_eq!(order(&moved(&r, "c", MoveTo::Top)), ["c", "a", "b"]);
    assert_eq!(order(&moved(&r, "a", MoveTo::Bottom)), ["b", "c", "a"]);
    assert_eq!(order(&moved(&r, "a", MoveTo::Up)), ["a", "b", "c"]);
    assert_eq!(order(&moved(&r, "b", MoveTo::Down)), ["a", "c", "b"]);
    assert_eq!(order(&moved(&r, "a", MoveTo::Position(2))), ["b", "a", "c"]);
    assert_eq!(order(&moved(&r, "c", MoveTo::Position(1))), ["c", "a", "b"]);
    assert_eq!(
        order(&moved(&r, "a", MoveTo::Position(99))),
        ["b", "c", "a"]
    );
    assert_eq!(order(&moved(&r, "b", MoveTo::Position(0))), ["b", "a", "c"]);
    assert_eq!(order(&without(&r, "b")), ["a", "c"]);
    assert!(with_all(&r, true).mods.iter().all(|m| m.enabled));
    assert!(with_enabled(&r, "b", true).mods[1].enabled);
    let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let r5 = reg(&["x", "c2", "y", "c1", "z"]);
    let arranged = arranged(&r5, &names(&["c1", "c2", "gone"]));
    assert_eq!(order(&arranged), ["x", "c1", "c2", "y", "z"]);
    assert!(arranged.mods[1].enabled && arranged.mods[2].enabled && !arranged.mods[0].enabled);
}
