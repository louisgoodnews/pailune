use super::*;

const PAGE: &str = "<html>\n<body>\n\t<div id=\"menu\" class=\"panel\">\n\t\t<img src=\"a.png\">\n\t\t<button id=\"ok\" class=\"btn disabled\">OK</button>\n\t\t<span class=\"hint\">old</span>\n\t</div>\n</body>\n</html>\n";

#[test]
fn element_and_comment_operations() {
    let patch = r##"<html-patch>
        <set at="#ok" class="+active -disabled" title="Confirm"/>
        <inner at="#menu .hint">new</inner>
        <append at="#menu"><p>added</p></append>
        <remove at="img[src=a.png]"/>
    </html-patch>
    <!-- @add-class selector="div.panel" class="wide" -->
    <!-- @replace selector="button#ok" --><button id="ok">Go</button><!-- @end -->"##;
    let out = apply(PAGE, patch);
    assert!(out.missed.is_empty(), "{:?}", out.missed);
    assert_eq!(
        out.text,
        "<html>\n<body>\n\t<div id=\"menu\" class=\"panel wide\">\n\t\t<button id=\"ok\">Go</button>\n\t\t<span class=\"hint\">new</span>\n\t\t<p>added</p>\n\t</div>\n</body>\n</html>\n"
    );
    assert_eq!(
        apply(PAGE, "<html-patch><remove at=\"#nope\"/></html-patch>").missed,
        ["#nope"]
    );
}

#[test]
fn void_elements_do_not_swallow_their_siblings() {
    let roots = dom::parse_html(PAGE);
    let found = select::select(&roots, "div button");
    assert_eq!(found.len(), 1, "the <img> closed itself");
    assert_eq!(select::select(&roots, "body #ok.btn").len(), 1);
}
