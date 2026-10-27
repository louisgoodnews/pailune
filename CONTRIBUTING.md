# Contributing to Pailune

Thanks for helping! Pailune is a mod manager for *Crimson Desert* — Linux first, with Windows and
macOS on the roadmap. This page explains how to set up, what we expect from changes, and how to
propose them. By taking part you agree to the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to help

- **Report a bug** or **suggest a feature** with the issue forms (*New issue* on GitHub).
- **Test real mods** and tell us which ones fail — attach the Log tab output or the log file
  (*Settings → Advanced → Open log folder*).
- **Improve documentation** in `docs/` (user guide, architecture, extension guides).
- **Write code**: pick an item from [docs/board.md](docs/board.md) (column *Next*) or an issue.
  For anything larger than a small fix, open an issue first so we can agree on the approach.

## Set up

```sh
git clone <repository> && cd pailune
cargo build            # Rust stable, edition 2024
cargo test             # offline; never touches your game
cargo run
```

Optional checks:

```sh
cargo test -- --ignored                          # real Nexus API / installed game / GPU screenshots
PYLUNE_SHOTS=/tmp/shots cargo test screens -- --ignored   # render every screen in both themes
cargo doc --no-deps --document-private-items --open
```

## What a change needs

Read [docs/architecture.md](docs/architecture.md) and [docs/extending.md](docs/extending.md);
the latter has recipes (new setting, action, job, mod file type, migration) and the conventions:

- Files stay **under 250 lines**; flat control flow (enums and `match`, early returns); data
  apart from logic; **rustdoc on every item**.
- The game folder is changed **only** through `symlink/` by exclusive jobs, links journaled
  first. Game archives are never edited in place.
- **Tests never touch a real installation** — use the fixtures in `src/tests/`.
- Interface changes follow [docs/ux-principles.md](docs/ux-principles.md) and are checked in both
  themes.
- New behaviour comes with a test; user-visible changes get a line in `CHANGELOG.md` and, if
  relevant, an update to the user guide or the in-app guide topics.

Before pushing, all three must pass (CI runs them too):

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Commits and pull requests

- One topic per pull request; describe *what* changed and *why*, and how you tested it (the PR
  template asks for this).
- Commit messages: a short imperative subject (`Add move-to-position for mods`), a blank line,
  then the reasoning. Reference board or issue IDs where they exist (`B-15`, `#42`).
- Keep the history readable; rebase on `main` instead of merging it in.

## Reverse-engineering and game data

Format knowledge goes into `docs/paz-format.md` (or a new `docs/*-format.md`). Never commit
files extracted from the game, save games, or other copyrighted assets — describe them instead.
Use `local/` (git-ignored) for samples.

## License

Pailune is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE). Unless you
state otherwise, any contribution you intentionally submit is licensed the same way, without
additional terms or conditions.
