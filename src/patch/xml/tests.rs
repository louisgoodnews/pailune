use super::*;

/// Shaped like the game's files: BOM, several roots, comments, `</>`.
const TEXTURES: &str = "\u{feff}<Texture Name=\"a\" Filename=\"UI/a.dds\" GetRect=\"1,1,70,70\"/>\n<Texture Name=\"b\" Filename=\"UI/b.dds\" GetRect=\"1,73,70,70\"/>\n";

const NESTED: &str = "\u{feff}<!-- do not edit <Prefab> by hand -->\n<PrefabObstacleData>\n\t<Prefab Name=\"/obj/bomb.prefab\">\n\t\t<Obstacle ShapeType=\"Box\" Scale=\"1.f\"/>\n\t</>\n\t<Prefab Name=\"/obj/rock.prefab\">\n\t\t<Obstacle ShapeType=\"Sphere\"/>\n\t</Prefab>\n</PrefabObstacleData>\n";

#[test]
fn parses_game_style_xml() {
    let roots = dom::parse(NESTED);
    assert_eq!(roots.len(), 1, "the comment's <Prefab> is not an element");
    let prefabs = &roots[0].children;
    assert_eq!(prefabs.len(), 2, "`</>` closed the first prefab");
    assert_eq!(prefabs[0].children[0].attr("Scale"), Some("1.f"));
    assert_eq!(dom::parse(TEXTURES).len(), 2);
}

#[test]
fn json_patches_edit_only_what_they_target() {
    let patch = r#"{"operations": [
        {"op": "set-attr", "xpath": "//Prefab[@Name='/obj/bomb.prefab']/Obstacle", "attribute": "Scale", "value": "2.f"},
        {"op": "remove", "xpath": "/PrefabObstacleData/Prefab[2]"},
        {"op": "add", "xpath": "//PrefabObstacleData", "value": "<Prefab Name=\"/obj/new.prefab\"/>"},
        {"op": "remove-attr", "xpath": "//Obstacle", "attribute": "ShapeType"},
        {"op": "remove", "xpath": "//Nothing"}
    ]}"#;
    let out = apply(NESTED, "x.xml.patch", patch).unwrap();
    assert_eq!(out.applied, 4);
    assert_eq!(out.missed.len(), 1);
    assert_eq!(
        out.text,
        "\u{feff}<!-- do not edit <Prefab> by hand -->\n<PrefabObstacleData>\n\t<Prefab Name=\"/obj/bomb.prefab\">\n\t\t<Obstacle Scale=\"2.f\"/>\n\t</>\n\t<Prefab Name=\"/obj/new.prefab\"/>\n</PrefabObstacleData>\n"
    );
}

#[test]
fn xml_patches_support_at_find_and_bulk_set() {
    let patch = r##"<xml-patch>
        <set find="Texture" key="b" GetRect="9,9,9,9" Filename="UI/c.dds"/>
        <insert-after find="Texture" key="a"><Texture Name="a2" Filename="UI/a2.dds"/></insert-after>
        <set target="//Texture[@Name='a']" attribute="Name" value="a&quot;1"/>
    </xml-patch>"##;
    let out = apply(TEXTURES, "t.xml.patch", patch).unwrap();
    assert!(out.missed.is_empty(), "{:?}", out.missed);
    assert_eq!(
        out.text,
        "\u{feff}<Texture Name=\"a&amp;quot;1\" Filename=\"UI/a.dds\" GetRect=\"1,1,70,70\"/>\n<Texture Name=\"a2\" Filename=\"UI/a2.dds\"/>\n<Texture Name=\"b\" Filename=\"UI/c.dds\" GetRect=\"9,9,9,9\"/>\n"
    );
    let ids = "<ui><button id=\"ok\" class=\"big red\"/></ui>";
    let by_id = apply(
        ids,
        "u.xml.patch",
        r##"<xml-patch><set at="#ok" text="Go"/><unset at=".red" attribute="class"/></xml-patch>"##,
    )
    .unwrap();
    assert_eq!(by_id.text, "<ui><button id=\"ok\" text=\"Go\"/></ui>");
    assert!(
        apply(
            ids,
            "u.xml.patch",
            "{\"operations\": [{\"op\": \"explode\", \"xpath\": \"//ui\"}]}"
        )
        .is_err()
    );
}

#[test]
fn merges_match_by_identity_and_append_the_rest() {
    let merge = "<xml-merge>\n<Texture Name=\"b\" GetRect=\"5,5,5,5\"/>\n<Texture Name=\"a\" __delete=\"true\"/>\n<Texture Name=\"c\" Filename=\"UI/c.dds\"/>\n</xml-merge>";
    let out = apply(TEXTURES, "t.xml.merge", merge).unwrap();
    assert_eq!(
        out.text,
        "\u{feff}<Texture Name=\"b\" Filename=\"UI/b.dds\" GetRect=\"5,5,5,5\"/>\n<Texture Name=\"c\" Filename=\"UI/c.dds\"/>\n"
    );
    // Nested: the root merges by tag, prefabs by Name.
    let nested = "<PrefabObstacleData><Prefab Name=\"/obj/rock.prefab\"><Obstacle ShapeType=\"Capsule\"/></Prefab></PrefabObstacleData>";
    let out = apply(NESTED, "o.xml.merge", nested).unwrap();
    assert!(
        out.text.contains("<Obstacle ShapeType=\"Capsule\"/>"),
        "{}",
        out.text
    );
    assert!(
        out.text
            .contains("<Obstacle ShapeType=\"Box\" Scale=\"1.f\"/>"),
        "untouched prefab kept"
    );
}

#[test]
fn quotes_and_unicode_in_tags_are_safe() {
    let base = "<R>\n  <C E=\"a > b\" Key=\"x\"/>\n  <D Key=\"y\"/>\n</R>\n";
    let roots = dom::parse(base);
    assert_eq!(
        roots[0].children.len(),
        2,
        "the quoted > does not end the tag"
    );
    let out = apply(
        base,
        "r.xml.patch",
        r#"{"operations":[{"op":"remove","xpath":"//C"}]}"#,
    )
    .unwrap();
    assert_eq!(out.text, "<R>\n  <D Key=\"y\"/>\n</R>\n");
    // Must not panic on a valueless attribute before a multi-byte character.
    let odd = dom::parse("<a foo ÿbar=\"x\">");
    assert_eq!(odd[0].attr("ÿbar"), Some("x"));
}

#[test]
fn unsupported_predicates_match_nothing_and_positions_count_per_parent() {
    let base =
        "<R><G><Item Key=\"a\"/><Item Key=\"b\"/></G><G><Item Key=\"c\"/><Item Key=\"d\"/></G></R>";
    let none = apply(
        base,
        "x.xml.patch",
        r#"{"operations":[{"op":"remove","xpath":"//Item[Name='Sword']"}]}"#,
    )
    .unwrap();
    assert_eq!((none.text.as_str(), none.missed.len()), (base, 1));
    let second = apply(
        base,
        "x.xml.patch",
        r#"{"operations":[{"op":"remove","xpath":"//Item[2]"}]}"#,
    )
    .unwrap();
    assert_eq!(
        second.text,
        "<R><G><Item Key=\"a\"/></G><G><Item Key=\"c\"/></G></R>"
    );
}

#[test]
fn merges_open_self_closing_elements_and_bad_ops_are_refused() {
    let base = "<R>\n<Item Key=\"a\" N=\"1\"/>\n</R>\n";
    let out = merge::merge(base, "<R><Item Key=\"a\" N=\"2\"><Sub/></Item></R>");
    assert_eq!(
        out.text,
        "<R>\n<Item Key=\"a\" N=\"2\"><Sub/></Item>\n</R>\n"
    );
    assert!(
        apply(
            base,
            "x.xml.patch",
            r#"{"operations":[{"op":"set-attr","xpath":"//Item","value":"v"}]}"#
        )
        .is_err()
    );
    assert!(
        apply(
            base,
            "x.xml.patch",
            "<xml-patch><unset target=\"//Item\"/></xml-patch>"
        )
        .is_err()
    );
    // Must not panic on an empty comment.
    let _ = crate::patch::html::apply("<p/>", "<!--><!--->");
}
