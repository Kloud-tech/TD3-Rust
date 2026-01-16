# Loglyzer (TD3-Rust)

Rust CLI to analyze log files: filtering, stats, and multi-format export.
This project demonstrates clean and performant log parsing, with a focus on
CLI UX, robustness, and readable results.

## Key features

- Filtering: level (ERROR), text search, time window (since/until)
- Statistics: total, per level, top errors, errors by hour, error rate by hour
- Outputs: text tables, JSON, CSV
- Performance: parallel parsing for large files + progress bar
- Robustness: invalid lines counted, clear error messages, exit codes

## Quick usage

```bash
# Basic analysis
cargo run -- sample.log

# Errors only + top 10
cargo run -- --errors-only --top 10 sample.log

# Text search + JSON
cargo run -- --search "database" --format json sample.log

# Time filtering
cargo run -- --since "2024-01-15 10:30:00" --until "2024-01-15 11:00:00" sample.log

# File output + verbose mode
cargo run -- --output result.json --format json --verbose sample.log

# Force parallel mode
cargo run -- --parallel sample.log
```

Release binary:

```bash
cargo build --release
./target/release/TD3-Rust sample.log
```

## Supported log format

```
YYYY-MM-DD HH:MM:SS [LEVEL] Message
```

Example:

```
2024-01-15 10:31:15 [ERROR] Failed to connect to API: timeout
```

Recognized levels: INFO, WARNING (or WARN), ERROR, DEBUG.

## Technical highlights

- CLI with clap (argument validation and help text)
- Regex parsing + chrono for timestamps
- Parallel processing with rayon for large files
- JSON/CSV export with serde
- Readable text tables (prettytable) + colored levels

## Tests

```bash
cargo test
```

## Project structure

- `src/main.rs`: CLI, parsing, filtering, analysis, rendering
- `tests/cli.rs`: CLI integration tests
- `sample.log`: demo file

## Context

Academic Rust project focused on software quality: performance, robustness,
and command-line user experience.
