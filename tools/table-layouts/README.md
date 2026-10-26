# Table-layout research tool

Derives the layouts in `data/table-layouts.json` from the installed game
(`docs/research/table-layouts.md`). Run it again after a game update; the
`shipped_table_layouts_match_the_game` test tells when that is needed.

```sh
mkdir -p /tmp/layouts/tables && cd /tmp/layouts
PYLUNE_TABLES_OUT=/tmp/layouts/tables cargo test export_tables_for_layout_research -- --ignored
python3 <repo>/tools/table-layouts/names.py ".../bin64/CrimsonDesert.exe" names.json
PYLUNE_LAYOUT_WORK=/tmp/layouts python3 <repo>/tools/table-layouts/export.py <repo>/data/table-layouts.json
cargo test shipped_table_layouts -- --ignored     # verify in Pylune itself
```

* `names.py` — field names from the executable's data sections (no code analysis).
* `tbl.py` — reads table indexes and records.
* `dp.py`, `infer.py` — field-type candidates and value checks.
* `mdl.py` — enumerates every layout that decodes all sampled records exactly, merges
  equivalent states, picks the minimum-description-length layout and reports per-field
  margins; a field's offset is *proven* when all exact layouts agree on it.
* `engine.py` — recursive version for nested classes (lists of structures, inline structures);
  solves element layouts where the parent search reaches the field.
* `export.py` — writes layouts that decode **every** record exactly, with evidence
  thresholds (≥ 10 records, no opaque blocks) and name certainty (unique shape in its class).

The exported tables and `names.json` are game data: keep them out of the repository.
