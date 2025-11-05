# POSIX Builtin Expansion Roadmap

## Roadmap
- Inventory current builtins (`alias`, `which`, upcoming map registry) against POSIX 2.9.1 compliance, documenting behaviour before changes.
- Extract a reusable shell context bundle (environment view, alias table, job metadata, I/O handles) so every builtin mirrors the dependency-injected struct model used by `Which`.
- Expand `BuiltinMap` registration to enumerate all POSIX-mandated builtins, capturing option metadata while keeping dispatch backward compatible.
- Implement builtins in focused batches—command discovery, environment/session control, job control, arithmetic/tests, and input/output—reusing shared helpers for option parsing, field splitting, and diagnostics.
- After each batch, run conformance-driven integration suites (positive, edge, locale-aware, failure cases) and sync documentation/help output with POSIX synopsis text.

## Cross-Cutting Workstreams
- Build a POSIX-compliant argument/option parsing layer supporting `getopts` semantics, escaped operands, and assignment detection for reuse across builtins.
- Centralize environment variable semantics (e.g., `HOME`, `OLDPWD`, `PWD`, `PATH`, `CDPATH`, `MAILCHECK`, `IFS`, `OPTARG`, `OPTIND`) so stateful builtins (`cd`, `read`, `getopts`, `umask`) can mutate shell state consistently.
- Introduce job-control primitives (process table, foreground/background tracking, signal forwarding) to support `bg`, `fg`, `jobs`, `kill`, and `wait` while exposing hooks to builtin structs.
- Provide filesystem/path utilities (logical versus physical resolution, permission checks, globbing hooks) required by `cd`, `pwd`, `test`, `hash`, `ulimit`, and `umask`.
- Define a diagnostics helper that emits POSIX-formatted messages, honours `set -e`/`-u`, locale (`LC_MESSAGES`), and exit-status rules for compound commands.

## Builtin Implementation Batches
- **Command identification:** Implement `command`, `type`, `hash`, and `unalias` as `Builtin` structs reusing alias/PATH helpers from `Which`, covering POSIX options (`-p`, `-t`, `-f`, etc.).
- **Directory and session control:** Implement `cd` (with `-L/-P`, `CDPATH`, `OLDPWD`), `pwd`, `umask`, `ulimit`, `times`, ensuring environment updates and exit codes match the specification.
- **Shell option and state:** Implement `set`, `unset`, `readonly`, `export`, `shift`, `getopts`, `trap`, `eval`, `exec`; wire them into the shell context for variable, function, and signal management.
- **Job control:** Implement `bg`, `fg`, `jobs`, `kill`, `wait`, `disown` (if adopted) using the shared job table and signal dispatcher, matching POSIX output formats and behaviours.
- **Input and data:** Implement `read`, `printf`, `echo` (with `-n`, escape handling), `test`/`[`, arithmetic builtins (`let`, `(( ))` if targeted) leveraging the common parser and word-expansion helpers.
- **History and alias enhancements:** Add `fc` (edit/re-execute) and enrich `alias`/`unalias` handling, integrating completion/history modules to satisfy POSIX notes.

## Validation Strategy
- For each builtin, create unit tests that call the struct with injected context (mirroring `Which`) and integration tests that execute scripted command lines via the control loop, asserting POSIX-compliant behaviour.
- Add compliance fixtures derived from specification tables (options, operands, diagnostics) and run them under varied environments (different `IFS`, locales, PATH settings).
- Use temporary directories and mocked job tables to validate filesystem and job-control semantics safely.
- Maintain a compliance matrix mapping specification clauses to tests and implementations; fail CI when regressions or coverage gaps appear.
- Document supported/unsupported extensions in `docs/`, updating help/usage output so users can verify conformance quickly.
