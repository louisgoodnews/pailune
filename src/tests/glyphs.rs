//! Every symbol used in the UI must exist in the bundled fonts; otherwise it
//! shows as a box on systems without matching fonts.

use eframe::egui::{self, FontId};
use std::collections::BTreeSet;
use std::path::Path;

fn sources(dir: &Path) -> Vec<String> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .collect()
}

#[test]
fn ui_symbols_have_glyphs() {
    let ui_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    let symbols: BTreeSet<char> = sources(&ui_dir)
        .iter()
        .flat_map(|s| s.lines())
        .filter(|l| !l.trim_start().starts_with("//"))
        .flat_map(str::chars)
        .filter(|c| !c.is_ascii())
        .collect();
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
    output.textures_delta.clear();
    let font = FontId::proportional(14.0);
    let missing: Vec<String> = ctx.fonts_mut(|f| {
        symbols
            .iter()
            .filter(|c| !f.has_glyph(&font, **c))
            .map(|c| format!("{c} U+{:04X}", *c as u32))
            .collect()
    });
    assert!(missing.is_empty(), "no glyph for: {missing:?}");
}
