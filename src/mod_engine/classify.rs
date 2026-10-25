//! Derives a [`ModKind`] from the placements of a mod's files.

use super::model::ModKind;
use super::plan::{PazFormat, Placement};

/// Folds file placements into a single [`ModKind`].
pub fn classify(placements: impl IntoIterator<Item = Placement>) -> ModKind {
    placements
        .into_iter()
        .filter_map(kind_of)
        .fold(ModKind::Empty, combine)
}

/// The kind a single placement contributes, if any.
fn kind_of(place: Placement) -> Option<ModKind> {
    match place {
        Placement::Mirror => Some(ModKind::Overlay),
        Placement::Bin64 => Some(ModKind::Asi),
        Placement::NeedsPaz(PazFormat::JsonPatch) => Some(ModKind::Json),
        Placement::NeedsPaz(PazFormat::Raw | PazFormat::Loose) => Some(ModKind::Raw),
        Placement::Skip => None,
    }
}

/// Combines two kinds; differing concrete kinds become [`ModKind::Mixed`].
fn combine(acc: ModKind, next: ModKind) -> ModKind {
    match (acc, next) {
        (ModKind::Empty, kind) => kind,
        (a, b) if a == b => a,
        _ => ModKind::Mixed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_placement() {
        assert_eq!(classify([]), ModKind::Empty);
        assert_eq!(classify([Placement::Bin64, Placement::Skip]), ModKind::Asi);
        assert_eq!(
            classify([Placement::NeedsPaz(PazFormat::JsonPatch); 3]),
            ModKind::Json
        );
        assert_eq!(
            classify([Placement::Mirror, Placement::NeedsPaz(PazFormat::Raw)]),
            ModKind::Mixed
        );
    }
}
