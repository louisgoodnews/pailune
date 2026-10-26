//! Virtual paths of the game's data tables: extension aliases from game
//! updates and the index (header) that belongs to each body.

/// Old and new extensions of table files (renamed in a game update).
const ALIASES: [(&str, &str); 2] = [
    (".pabgb", ".staticinfobody"),
    (".pabgh", ".staticinfoheader"),
];

/// The path plus its renamed-extension alias.
pub fn variants(path: &str) -> Vec<String> {
    let lower = path.to_lowercase().replace('\\', "/");
    let swapped = ALIASES.iter().find_map(|(a, b)| {
        lower
            .strip_suffix(a)
            .map(|s| format!("{s}{b}"))
            .or_else(|| lower.strip_suffix(b).map(|s| format!("{s}{a}")))
    });
    std::iter::once(lower).chain(swapped).collect()
}

/// The table index belonging to a table body, if `vpath` is one.
pub fn header_of(vpath: &str) -> Option<String> {
    [
        (".pabgb", ".pabgh"),
        (".staticinfobody", ".staticinfoheader"),
    ]
    .iter()
    .find_map(|(body, head)| vpath.strip_suffix(body).map(|s| format!("{s}{head}")))
}

/// File name of a virtual path.
pub fn name_of(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}
