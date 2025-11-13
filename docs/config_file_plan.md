# Iridium Configuration File Plan

## Goals
- Introduce a YAML configuration file located at `~/.iridiumrc` that centralizes user-defined settings for the shell.
- Allow users to configure the default persistence database path (replacing the current environment-variable-only approach).
- Keep the design extensible so future ControlState options (e.g., encryption defaults, auto-save intervals, UI preferences) can be added without breaking compatibility.
- Maintain a minimal runtime footprint: configuration is parsed once during startup and injected wherever needed.

## File Format & Location
- **Path Resolution**
  1. Look for `IRIDIUM_CONFIG` env var first; if set, treat it as an absolute/relative override.
  2. Otherwise default to `$HOME/.iridiumrc`.
  3. If the file does not exist, proceed with built-in defaults.
- **Format**: YAML 1.2 (use a lightweight parser such as `serde_yaml`).
- **Structure**: Top-level mapping with logical sections, e.g.
  ```yaml
  persistence:
    database_path: "~/.local/share/iridium/buffers.db"
    encrypt: true
    algorithm: "chacha20poly1305"
    compression: "lz4"
  control:
    auto_save_interval_ms: 30000
  ui:
    prompt_theme: "default"
  ```
- Keep keys snake_case to match Rust struct fields (for `serde` derive).

## Data Model
```rust
#[derive(Deserialize, Default)]
pub struct IridiumConfig {
    #[serde(default)]
    pub persistence: PersistenceConfigSection,
    #[serde(default)]
    pub control: ControlConfigSection,
    #[serde(default)]
    pub ui: UiConfigSection,
}

#[derive(Deserialize, Default)]
pub struct PersistenceConfigSection {
    pub database_path: Option<PathBuf>,
    pub encrypt: Option<bool>,
    pub algorithm: Option<String>,
    pub key_file: Option<PathBuf>,
    pub passphrase: Option<String>,
    pub pbkdf2_iterations: Option<u32>,
}

#[derive(Deserialize, Default)]
pub struct ControlConfigSection {
    pub auto_save_interval_ms: Option<u64>,
    pub default_buffer_mode: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct UiConfigSection {
    pub prompt_theme: Option<String>,
}
```
- Each nested section uses `Option` fields so unspecified settings fall back to existing defaults.
- Unknown keys should be tolerated; log a warning to aid debugging.

## Load & Merge Strategy
1. **Read File Once**: At `ControlState::new`, call `config::load_user_config() -> IridiumConfig`.
2. **Merge Order**:
   - Built-in defaults (compiled in).
   - YAML file values (if present).
   - Environment variables (highest precedence for sensitive overrides such as encryption keys).
3. **Injection Points**:
   - Persistence path: `persistence.database_path` feeds into `PersistenceConfig` before evaluating env vars.
   - Encryption toggles: config fields provide defaults; env vars still allow ad-hoc overrides.
   - Control/UI sections feed future knobs (e.g., `auto_save_interval_ms` driving timers inside `ControlState`).
4. **Validation**: After merging, validate paths (expand `~`), ensure directories are writable, and warn instead of aborting when possible.

## Extensibility Considerations
- Use `serde(deny_unknown_fields)` only on leaf structs where necessary; otherwise log-and-ignore to support forward compatibility.
- Keep the config module isolated (`src/config.rs` or `src/config/mod.rs`) so new sections can be added without touching unrelated code.
- Provide helper getters that apply fallback logic (e.g., `fn persistence_path(&self) -> PathBuf`) to keep `ControlState` simple.
- Consider watch/reload support later by emitting events when the config file changes.

## Developer Tasks Breakdown
1. **Introduce `conf` Module**
   - Add `serde` + `serde_yaml` (and any path utilities) to `Cargo.toml`.
   - Create `src/conf/mod.rs` housing the data models (`IridiumConfig`, section structs) plus a `loader` function.
   - Keep helpers such as `expand_user_path`, env override logic, and future section modules under `src/conf/`.
2. **Persistence Integration**
   - Replace `PersistenceConfig::from_env()` with `PersistenceConfig::from_sources(config: Option<&IridiumConfig>)`.
   - Honor `persistence.database_path` from the YAML file (after `~` expansion) before falling back to env/default path.
3. **Future Control Hooks**
   - Store the loaded `IridiumConfig` inside `ControlState` (or behind `Arc`) so future lifetime knobs can tap into it.
   - Document how new sections/fields should be added so future contributors follow the same pattern.
4. **Testing & Docs**
   - Unit tests covering path precedence, malformed YAML handling, and default fallbacks.
   - Update README/docs to mention `~/.iridiumrc` and show sample snippets.

## Open Questions
- Should we support per-project config (e.g., `.iridiumrc` in the CWD) that overrides the home file?
- Do we need encryption/keychain support for secrets embedded in the config, or should such settings remain env-only?
- Should the config loader watch for changes or require a manual restart to pick up new values?
