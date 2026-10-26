# Extending Pylune

Recipes for the changes contributors make most often, and the conventions every change follows.
Read [architecture.md](architecture.md) first.

## Conventions

- **Files under 250 lines.** Split by responsibility (`foo/mod.rs` + `foo/bar.rs`); tests move
  to a `tests.rs` submodule when a file grows.
- **Flat control flow.** Prefer enums and `match` (also on tuples) over nested `if`; early
  returns over arrow code; iterator chains over index loops.
- **Data apart from logic.** Static content (guide topics, migrations, settings categories)
  lives in `const` tables; functions compute plans from data, and a separate step executes them.
- **Rustdoc on every item** outside tests, explaining *why* where it is not obvious.
- **Errors:** return `crate::error::Result`; attach paths to I/O errors with `.at(path)`; user-
  facing messages say what happened and what to do next ([ux-principles.md](ux-principles.md) R-08).
- **Safety:** the game folder is changed only by exclusive jobs, only through `symlink/` (links
  journaled first), and never by editing game files in place.
- **Tests never touch the real installation.** Use the fixtures in `src/tests/mod.rs` (fake game,
  Steam library and data folders in a temp dir). Tests that read the real game are `#[ignore]`.
- **Before committing:** `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.

## Add a setting

1. Add the field to the right section in `src/config/prefs.rs` (or `config/mod.rs` for
   `[game]`, `[nexus]`, `[launch]`, `[jobs]`, `[logging]`) with rustdoc and a default in its
   `Default` impl. `#[serde(default)]` keeps old files valid.
2. Show it on its page in `src/ui/views/settings/` with `setting(ui, title, help, |ui| …)`: a
   constrained widget (checkbox, choice, `DragValue` with `range`) and a one-line help text.
3. If it needs validation, add it to `SettingsDraft::to_config` (`src/ui/draft.rs`); if it
   belongs to a resettable group, check `SettingsDraft::reset`.
4. Read it where it takes effect (jobs see `ctx.config`, the UI `self.config`).
5. Document it in the `config.toml` table of [data-model.md](data-model.md) and test it
   (`src/tests/ui_prefs.rs`).

## Add a user action

1. Add a variant to the matching intent enum in `src/ui/intent.rs` (or a new enum plus a variant
   of `Action`).
2. Handle it in `src/ui/actions/<feature>.rs`, returning `Result<Option<StatusLine>>`, and route it
   in `PyluneApp::dispatch` (`ui/actions/mod.rs`).
3. Emit it from a view: views return `Vec<Action>`, typically via `clicked([(button, action)])`.
4. Destructive and not undoable? Add a `Confirm` variant and its text in `ui/views/confirm.rs`
   (button named after the action). Undoable? Prefer undo over a dialog.
5. Test it headlessly: build the app with `tests::ui::app(&fixture())`, `dispatch`, `settle`.

## Add a background job

1. Add a `JobKind` variant (`src/jobs/mod.rs`) with a title, and decide its `access()`:
   `Exclusive` if it touches the game folder or Pylune's state as a whole, else `Shared`.
2. Implement it in `src/jobs/tasks/<area>.rs` as `fn(ctx: &JobContext, …) -> Result<JobOutput>`;
   use `ctx.db()`, `ctx.progress(done, total)`, `ctx.journal(conn, Event::…)`.
3. Route it in `jobs/tasks/mod.rs`. Follow-ups after success go into `after_done` in
   `ui/actions/poll.rs`.

## Support a new kind of mod file

1. Teach `mod_engine/placement.rs::placement` the path pattern, returning an existing
   `Placement` or a new variant (with a `note()` for logs and the file list).
2. If it is linked: `mod_engine/plan.rs` turns placements into `LinkOp`s.
   If it goes into the archives: collect it in `overlay/collect.rs` and pack or apply it in
   `overlay/build.rs` / `overlay/patches.rs`; format code belongs in `paz/` or `patch/`.
3. Conflicts: make sure `mod_engine/conflicts.rs` sees the same target paths.
4. Add a fixture mod to `src/tests/` and an overlay or deployment test; if the format is
   reverse-engineered, document it in [paz-format.md](paz-format.md).

## Change the database

Append a migration string to `MIGRATIONS` in `src/db/schema.rs` (never edit an old one), read
and write the new columns in the matching `db/*.rs` module, and extend the upgrade test. Backups
restore older databases through the same migrations.

## Add a guide topic or onboarding step

Topics are data in `src/ui/guide/topics.rs` (`title`, search `keywords`, short `body`
paragraphs that name real buttons). Steps live in `src/ui/guide/steps.rs` with a `Facts` check.
Keep answers task-oriented and short (R-20).

## Add a command-line command and MCP tool

1. Add a variant to `Op` (`src/cli/ops.rs`); mark it in `read_only()` or `destructive()` if so.
2. Read-only: implement it in `cli/read.rs`. Changing: map it to UI actions in `actions()` in
   `cli/engine.rs` (or a dedicated function, like `export`/`import`) — never reimplement logic.
3. Add the clap subcommand and its mapping in `cli/mod.rs`.
4. Add the tool to `mcp/tools/table.rs` (name, description, parameters, hints) and its mapping in
   `mcp/tools/mod.rs::op`; extend `mcp/tests.rs`.

## Add a Nexus API call

Add a method to `nexus::api::Client` (it applies rate-limit bookkeeping and redacts the key),
a response type in `nexus/mod.rs`, a job that calls it, and a case for the mock server in
`src/tests/nexus_http.rs`.

## Port to another platform

All OS differences live in `src/platform/` (one file per OS, same function set, chosen with
`cfg(target_os)`). Fill in or fix that file rather than adding `cfg` elsewhere. Deployment
without symlinks is solved (decision D-11: stamped hard links or copies); what remains for
Windows is the `nxm://` registration (registry keys) and packaging.

## Interface changes

Follow [ux-principles.md](ux-principles.md). Use `theme::accent()`, `danger()`, `success()`,
`warning()`, `muted()` (never fixed colours), check new symbols with the glyph test, and review
the result in both themes: `PYLUNE_SHOTS=/tmp/shots cargo test screens -- --ignored`.
