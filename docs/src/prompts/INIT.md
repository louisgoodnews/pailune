# Initialization Prompt: Pailune - Crimson Desert Linux Mod Manager

You are tasked with building a native Linux mod manager named **Pailune** (named after Pailune, the game's first explorable region) for the game **Crimson Desert** (running via Steam Proton) using **Rust** and a GUI built with **Egui** (via `eframe`).

## Initial Interaction Requirement
Upon startup and before generating the core implementation, you must **explicitly ask the user** to provide:
1. The full path to their *Crimson Desert* installation (or Steam library / Proton prefix).
2. Their Steam ID (if required for path resolution or configuration).

Wait for the user's input regarding these paths before finalizing the setup script generation.

---

## Core Architecture & File Handling Philosophy

Pailune must adhere strictly to a non-destructive file management approach:
- **Staging Folder:** Pailune will keep all installed and managed mods in a dedicated local staging folder.
- **Symlink Deployment:** Pailune deploys mods exclusively via **symbolic links (symlinks)**. It never touches or modifies original game data files directly.
- **Safe Backups (`.bak`):** The original target file/directory structures (such as core mod directories or metadata) are archived as `.bak` files to allow instant, reliable reverts.

---

## Technical & Architectural Requirements

To ensure high code quality, maintainability, and readability, you must strictly follow these rules:

- **File Length:** Every `.rs` file must be kept small and strictly **under 250 lines**.
- **Minimal Nesting:** Avoid deep `if/else` ladders. Use **Enums with pattern matching (`match`)** to handle control flows and states flatly.
- **Functional Paradigm:** Prioritize functional programming principles (pure functions where possible, iterator methods like `.map()`, `.filter()`, `.fold()` instead of imperative loops).
- **Data-Oriented Design:** Strictly separate data structures (pure `struct` definitions without embedded business logic) from the business logic (`impl` blocks and free functions).
- **Leverage Existing Crates:** Import established crates from the Rust ecosystem (e.g., `serde`, `serde_json`, `walkdir`, `rfd` for native file dialogs) to avoid reinventing the wheel.
- **High Modularity:** Organize the codebase cleanly into modules (e.g., `ui`, `steam`, `mod_engine`, `config`, `symlink`), avoiding monolithic structures. Never dump everything into `main.rs`.
- **Documentation:** Document modules, public functions, and complex logic thoroughly using Rustdoc (`///`).

---

## Functional Scope (Crimson Desert Modding Mechanics)

The tool must manage the game's specific modding architecture:
1. **Backup & Reset:** Backup original game directory targets as `.bak` files (e.g., handling metadata like `0.papgt` and directory structures > 35) to provide a "Clean Slate" revert mechanism.
2. **JSON Mod Management:** Manage JSON-based mods via the staging folder and deploy them using symlinks.
3. **ASI Mod Support:** Deploy ASI mod files and DLLs (such as `version.dll`) into the correct `bin64/` subdirectory of the Proton prefix using symlinks or safe staging references.
4. **GUI Design:** A clean, modern dark-mode interface with clear status indicators, checkboxes for active mods, and a prominent reset/revert button.
