# BufferStore Persistence Plan

## Objectives & Constraints
- Load a binary on-disk database into the `ControlState`'s `BufferStore` at startup and reconstruct every buffer (name, open/dirty flags, contents).
- Before the shell loop exits (normal quit, Ctrl+C, EOF) write the full set of buffers back to disk in the binary format.
- Keep the persistence pathway pluggable so compression and encryption layers can be added later without redesigning the core.
- Preserve thread safety: all read/write access goes through the existing `Arc<Mutex<BufferStore>>`.
- Fail safe: corrupted persistence data should never crash the shell or mutate buffers; errors are logged and the session continues with in-memory data.

## Architectural Overview
1. **Configuration**
   - Add `PersistenceConfig` (path, format version, optional compression/encryption selections).
   - Default path: `~/.local/share/iridium/buffers.db` (overridable via env/CLI flag in the future).
2. **Snapshot Layer**
   - Introduce `BufferSnapshot` as a serializable view (`name`, `requires_name`, `is_open`, `dirty`, `Vec<String>` lines).
   - `BufferStore` gains helper `fn snapshots(&self) -> Vec<BufferSnapshot>` and `fn hydrate(&mut self, Vec<BufferSnapshot>)`.
3. **Binary Persistence Engine**
   - Module `store::persistence` owning `BinaryBufferDb`.
   - Entry point `BufferDb::load(config: &PersistenceConfig) -> io::Result<Vec<BufferSnapshot>>` and `BufferDb::store(config, &[BufferSnapshot])`.
   - Reads/writes through a `PersistencePipeline` chain (see Extensions below).
4. **ControlState Integration**
   - `ControlState::new` creates a `PersistenceManager` that immediately loads snapshots and hydrates `BufferStore`.
   - `PersistenceManager` holds `Arc<Mutex<BufferStore>>` + `PersistenceConfig`.
   - Expose `ControlState::flush_persistence(&self) -> io::Result<()>` (or `Result<(), PersistenceError>`).
   - `control::run_loop_with_editor` calls `flush_persistence` whenever the loop breaks (EXIT, Ctrl+C, EOF). Additionally, guard with `Drop` to cover panic paths.

## Binary File Layout (Byte-Aligned)
All multi-byte integers use little-endian encoding. Booleans are stored as full `u8` values to keep field boundaries byte-aligned, and per-record padding makes every structured chunk a multiple of eight bytes for easy streaming into future SIMD/compression layers.
```
Header (32 bytes total):
- magic: [u8; 8]  -> b"IRDBUF\0\0" for 8-byte alignment
- version: u32    -> format version, start at 1
- flags: u32      -> bitmask for enabled persistence layers
- reserved0: u64  -> future use / checksum seed
- buffer_count: u64

Per buffer record (aligned to 24 + name bytes + line payload):
- name_len: u32
- line_count: u32
- requires_name: u8
- is_open: u8
- dirty: u8
- padding0: u8    -> keeps the control block multiple of 8 bytes
- padding1: u32   -> reserved for future flags, maintains alignment
- name bytes: name_len UTF-8 bytes (no terminator; already byte-aligned)
- per line:
    - line_len: u32
    - padding_line: u32 (reserved so each line header is 8 bytes)
    - line bytes: line_len UTF-8 bytes, followed by `padding_tail` bytes (0–7) so the next field starts on an 8-byte boundary. Padding bytes are zeroed but ignored on read.
```
- When writing, ensure `padding_tail = (8 - (line_len % 8)) % 8`; when reading, skip the zero padding after consuming each line.
- Wrap the entire payload (after the fixed-size header) with the compression/encryption pipeline when configured.
- Version bumps allow migration logic (e.g., change padding strategy, add checksums) while old binaries remain distinguishable via the header.

## Startup Flow
1. `ControlState::new` builds `PersistenceManager`.
2. Manager resolves file path and opens it if it exists.
3. Header validation (magic, version); unsupported version logs warning and aborts loading.
4. Run inverse pipeline: decrypt → decompress → binary decode.
5. Convert snapshots into actual `Buffer` instances via `BufferStore::hydrate`. Skip buffers that fail validation (e.g., invalid UTF-8).
6. Attach hydrated store to `Terminal` as today.

## Shutdown Flow
1. `run_loop_with_editor` detects loop termination and calls `control_state.flush_persistence()`.
2. `flush_persistence`:
   - Locks `BufferStore`, collects snapshots, and drops the lock before I/O.
   - Serializes to binary blob, runs pipeline (compress/encrypt), writes to temp file `buffers.db.tmp`.
   - `fsync` + atomic rename to target path for crash safety.
   - Surface errors to stderr but still allow process exit.
3. `PersistenceManager` implements `Drop` to invoke `flush_persistence` as a last resort if the caller forgot.

## Extension Hooks (Compression & Encryption)
- Define traits:
  ```rust
  pub trait PersistenceLayer {
      fn wrap_writer(&self, writer: Box<dyn Write>) -> Result<Box<dyn Write>>;
      fn wrap_reader(&self, reader: Box<dyn Read>) -> Result<Box<dyn Read>>;
      fn flag_bit(&self) -> u16;
  }
  ```
- `CompressionLayer` and `EncryptionLayer` implement `PersistenceLayer`.
- `PersistencePipeline` keeps an ordered `Vec<Box<dyn PersistenceLayer>>`.
- Header `flags` store which layers are active; loader replays them in reverse order.
- Initial implementation ships with `NoCompression` and `NoEncryption` placeholders so wiring is ready without third-party crates.

## Encryption Roadmap
1. **Key Management**
   - Support two modes: (a) passphrase -> HKDF-derived key, and (b) direct key material from `IRIDIUM_PERSIST_KEY_FILE`.
   - Store metadata (salt, KDF parameters, algorithm id) in the header `reserved0`/flag bits so the loader can derive keys deterministically.
   - Provide a helper CLI command (`:persist key rotate`) to regenerate salts and validate the configured key source.
2. **Algorithm Selection**
   - Start with AEAD (e.g., ChaCha20-Poly1305) for authenticated encryption; use `ring` or `aes-gcm` crate when network policy allows vendoring.
   - Reserve multiple flag bits: lower nibble = compression, upper nibble = encryption algorithm so combinations remain unique.
3. **Pipeline Integration**
   - Implement `AeadEncryptionLayer` that wraps writers/readers to encrypt after compression; on load, decrypt before decompressing.
   - Extend the header to include a per-file nonce and auth tag appended to the payload.
4. **Algorithm Matrix**
   - Define `EncryptionAlgorithm` enum (e.g., `ChaCha20Poly1305`, `Aes256Gcm`, `Disabled`) and map each variant to a unique flag bit.
   - Default to `ChaCha20Poly1305` because it is widely audited, constant-time on software-only targets, and performs well on both Intel and ARM without requiring AES-NI.
   - Support `Aes256Gcm` when AES-NI (or ARMv8 Cryptography Extensions) is detected; expose a `prefer_aes` toggle so deployments can opt-in when hardware acceleration is available.
   - Leave room for future algorithms (e.g., `XChaCha20Poly1305`) by reserving additional bits and validating unknown flags at load time.
5. **Compression Strategy**
   - Always compress serialized snapshots before encryption to shrink disk footprint and remove plaintext patterns.
   - Default algorithm: **LZ4 (lz4_flex)** — fast streaming encoder/decoder, minimal latency, no external dependencies.
   - Future options: `Zstd` (better ratios, slower), `Snappy` (balanced), `None` (for debugging). Reserve flag bits and config enumerations so additional codecs can be added without schema changes.
   - Header layout reserves 4 bits for compression ID (e.g., 0 = none, 1 = LZ4, 2 = Zstd, 3 = Snappy).
   - Compression happens on the entire snapshot payload; encryption wraps the compressed blob when enabled.
   - `~/.iridiumrc` gains `persistence.compression = "lz4"` and `IRIDIUM_PERSIST_COMPRESSION` env var can override per-run.
6. **Configuration Surface**
   - Extend `PersistenceConfig` with `PersistenceSecurity { encryption: EncryptionMode, key_source: KeySource }`.
   - Default to `EncryptionMode::Disabled`; allow enabling via env (`IRIDIUM_PERSIST_ENCRYPT=1`) or config file once available.
7. **Testing & Validation**
   - Add golden files encrypted with known keys to verify backward compatibility.
   - Include failure-path tests (wrong key, tampered auth tag) to ensure the loader rejects corrupted data gracefully.
   - Benchmark read/write overhead with and without encryption to ensure acceptable startup/exit latency.

## Implementation Steps
1. **Config & Types**
   - Add `config::persistence` module with `PersistenceConfig`, `BufferSnapshot`, and enums for compression/encryption algorithms.
2. **BufferStore Plumbing**
   - Implement snapshot/hydrate helpers plus lightweight iterator (`fn iter(&self) -> impl Iterator<Item=(&String, &Buffer)>`).
3. **Persistence Engine**
   - Create `BinaryBufferDb` with read/write helpers, header parsing, and atomic file write utilities.
   - Encode/decode functions purely on `BufferSnapshot` to keep `BufferStore` unaware of IO.
4. **Pipeline Abstractions**
   - Stub `CompressionLayer` + `EncryptionLayer` implementations returning passthrough readers/writers; wire flag bits.
5. **ControlState Integration**
   - Add `PersistenceManager` field to `ControlState`, ensuring it is initialized before buffers are hydrated and flushed on exit.
   - Extend `ControlSession` (or expose `ControlState::flush_persistence`) so `control::run_loop_with_editor` can trigger persistence after the loop.
6. **Error Handling & Logging**
   - Define `PersistenceError` enum; log warnings instead of panicking for IO/corruption issues.
7. **Tests**
   - Unit tests for encode/decode roundtrip, corrupted header rejection, snapshot ↔ buffer conversion.
   - Integration test: create `ControlState`, open buffers, call `flush_persistence`, reload, ensure buffers restored with flags.

## Testing & Validation Strategy
- Add `tests/persistence_roundtrip.rs` (uses temp dir) verifying binary file contents and atomic rename behavior.
- Use feature flag or env var to point persistence to a test directory.
- For future compression/encryption, add mock layer implementations used in tests to ensure pipeline order is respected.

## Configuring Encryption (Read & Write)
1. **Enable Encryption**
   - Set `IRIDIUM_PERSIST_ENCRYPT=1` (or add the equivalent config entry once a config file exists). Leaving this unset keeps plaintext mode.
2. **Choose Algorithm**
   - Optional `IRIDIUM_PERSIST_ALGO` values: `chacha20poly1305` (default, software-friendly) or `aes256gcm` (use when hardware AES acceleration is available). Unknown values are rejected at startup.
3. **Configure via `~/.iridiumrc`**
   - Set:
     ```yaml
     persistence:
       database_path: "~/.local/share/iridium/buffers.db"
       encrypt: true
       algorithm: "chacha20poly1305"
       key_file: "~/.config/iridium/key.hex"   # contains 64-char hex key
       passphrase: "optional-passphrase"       # alternatively derive via PBKDF2
       pbkdf2_iterations: 600000
     ```
   - Relative paths are resolved relative to the config file directory; `key_file` may point to a secret tracked outside of git.
4. **Provide Key Material (Env overrides still allowed)**
   - Use one of the mutually exclusive inputs:
     - `IRIDIUM_PERSIST_KEY=<64 hex chars>` for a raw 256-bit key.
     - `IRIDIUM_PERSIST_KEY_FILE=/path/to/key.hex` to load the hex key from disk.
     - `IRIDIUM_PERSIST_PASSPHRASE="your phrase"` to derive a key via PBKDF2; optionally override rounds with `IRIDIUM_PERSIST_PBKDF_ITERS` (defaults to 600k).
   - `IRIDIUM_PERSIST_COMPRESSION=lz4` (default) lets users switch codecs per run once additional algorithms are available.
5. **Runtime Behavior**
   - **Write path**: serialize snapshots → compress (always) → encrypt if configured → write header/payload. Salt and nonce metadata are persisted as part of the encrypted block.
   - **Read path**: load header → decrypt when required → decompress → hydrate `BufferStore`. On errors (bad key, corrupt compressed stream) log warnings and fall back to an empty store.
6. **Disabling / Overrides**
   - Use `IRIDIUM_DISABLE_PERSISTENCE=1` to bypass all disk IO (no reads, no writes) regardless of encryption settings.
   - Point the database file elsewhere via `IRIDIUM_BUFFER_DB_PATH=/custom/path/buffers.db` so encrypted states can live outside the default data dir.

## Open Questions / Follow-ups
- Where should the persistence path and options be surfaced to users (env var vs config file)?
- Do we need to persist additional metadata (cursor positions, dirty timestamps)? If so, bump format version.
- Consider opt-in auto-save interval to avoid losing work mid-session; plan can reuse the same persistence engine.
