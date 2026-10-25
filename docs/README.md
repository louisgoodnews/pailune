# Pylune documentation

| Document | For | Contents |
|----------|-----|----------|
| [user-guide.md](user-guide.md) | Players | Installing Pylune, first steps, everyday tasks, troubleshooting. |
| [vision.md](vision.md) | Everyone | Why Pylune exists, who it is for, goals and non-goals. |
| [architecture.md](architecture.md) | Contributors | Layers, modules, threads and processes, the main flows. |
| [data-model.md](data-model.md) | Contributors | Files on disk, `config.toml`, the SQLite schema, archive formats. |
| [extending.md](extending.md) | Contributors | Step-by-step guides for common extensions and the code conventions. |
| [decisions.md](decisions.md) | Contributors | Architecture decision records: what was chosen and why. |
| [ux-principles.md](ux-principles.md) | Contributors | Evidence-based interface rules (R-01 … R-29) and where they live. |
| [paz-format.md](paz-format.md) | Contributors | The game's archive formats as far as Pylune needs them. |
| [audit.md](audit.md) · [roadmap.md](roadmap.md) · [board.md](board.md) | Maintainers | Findings, plan and status of every work item. |

API documentation of the code itself: `cargo doc --no-deps --document-private-items --open`.
Every function and constant outside tests carries rustdoc.
