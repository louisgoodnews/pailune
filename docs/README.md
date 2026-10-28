# Pailune documentation

| Document | For | Contents |
|----------|-----|----------|
| [user-guide.md](user-guide.md) | Players | Installing Pailune, first steps, everyday tasks, troubleshooting. |
| [vision.md](vision.md) | Everyone | Why Pailune exists, who it is for, goals and non-goals. |
| [architecture.md](architecture.md) | Contributors | Layers, modules, threads and processes, the main flows. |
| [data-model.md](data-model.md) | Contributors | Files on disk, `config.toml`, the SQLite schema, archive formats. |
| [extending.md](extending.md) | Contributors | Step-by-step guides for common extensions and the code conventions. |
| [decisions.md](decisions.md) | Contributors | Architecture decision records: what was chosen and why. |
| [ux-principles.md](ux-principles.md) | Contributors | Evidence-based interface rules (R-01 … R-29) and where they live. |
| [save-format.md](save-format.md) | Contributors | The save-game container and decoding options. |
| [paz-format.md](paz-format.md) | Contributors | The game's archive formats as far as Pailune needs them. |
| [design/visual-mod-creator.md](design/visual-mod-creator.md) | Contributors | Visual node-based mod creator architecture and graph compiler. |
| [design/in-place.md](design/in-place.md) | Contributors | In-place deployment route design, safety guarantees, and versioning. |
| [research/engine-architecture.md](research/engine-architecture.md) | Contributors | BlackSpace Engine reverse engineering across 2M files and 36 pack groups. |
| [research/asi-mods.md](research/asi-mods.md) | Contributors | ASI loader mechanisms, pattern scanning, MinGW/Rust cross-compilation. |
| [research/table-layouts.md](research/table-layouts.md) | Contributors | Deriving the game's binary static info table layouts for format-3 mods. |
| [research/script-bridge.md](research/script-bridge.md) | Contributors | Whether scripts can run inside the game. |
| [audit.md](audit.md) · [roadmap.md](roadmap.md) · [board.md](board.md) · [priority-list.md](priority-list.md) | Maintainers | Findings, plan and status of every work item. |

API documentation of the code itself: `cargo doc --no-deps --document-private-items --open`.
Every function and constant outside tests carries rustdoc.
