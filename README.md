# TD3-Rust

Add to `Cargo.toml`:

```toml
[dependencies]
clap = { version = "4.5.51", features = ["derive"] }
```

* **Exercise**:
  * Run the program with `--help` and observe the generated help text
  * Add a `--top` argument that accepts a number (e.g., `--top 10` for top 10 results)
  * Add a `--search` argument to filter logs containing specific text
  * Test with invalid arguments and observe error messages

---

### **Part 2 – File I/O & Log Parsing (50 min)**

**Building Block**: Reading and parsing log data

* **Concepts**:

  * Efficient file reading with `BufReader`
  * Line-by-line processing for large files
  * Pattern matching with regex
  * Parsing structured log formats (common patterns: timestamp, level, message)
* **Demo Code**: Parse log entries

```rust
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone)]
struct LogEntry {
    timestamp: String,
    level: LogLevel,
    message: String,
}

#[derive(Debug, Clone, PartialEq)]
enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

impl LogLevel {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "INFO" => Some(LogLevel::Info),
            "WARN" | "WARNING" => Some(LogLevel::Warning),
            "ERROR" => Some(LogLevel::Error),
            "DEBUG" => Some(LogLevel::Debug),
            _ => None,
        }
    }
}

fn parse_log_line(line: &str) -> Option<LogEntry> {
    // Example log format: "2024-01-15 10:30:45 [INFO] Application started"
    let re = Regex::new(r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})\s+\[(\w+)\]\s+(.+)$")
        .unwrap();

    re.captures(line).and_then(|caps| {
        Some(LogEntry {
            timestamp: caps.get(1)?.as_str().to_string(),
            level: LogLevel::from_str(caps.get(2)?.as_str())?,
            message: caps.get(3)?.as_str().to_string(),
        })
    })
}

fn read_logs(path: &Path) -> Result<Vec<LogEntry>, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(entry) = parse_log_line(&line) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entries = read_logs(Path::new("sample.log"))?;

    println!("Parsed {} log entries", entries.len());

    // Count by level
    let error_count = entries.iter()
        .filter(|e| e.level == LogLevel::Error)
        .count();

    println!("Errors: {}", error_count);

    Ok(())
}
```

Add to `Cargo.toml`:

```toml
regex = "1.12.2"
```

**Sample log file** (`sample.log`):

```
2024-01-15 10:30:45 [INFO] Application started
2024-01-15 10:30:46 [DEBUG] Loading configuration from config.yml
2024-01-15 10:30:47 [INFO] Database connection established
2024-01-15 10:31:02 [WARNING] High memory usage detected: 85%
2024-01-15 10:31:15 [ERROR] Failed to connect to API: timeout
2024-01-15 10:31:16 [INFO] Retrying API connection...
2024-01-15 10:31:18 [INFO] API connection successful
2024-01-15 10:32:00 [ERROR] Database query failed: syntax error
2024-01-15 10:32:01 [WARNING] Cache miss for key: user_1234
2024-01-15 10:33:00 [INFO] Processing completed successfully
```

* **Exercise**:
  * Integrate `read_logs()` with your CLI from Part 1
  * Implement the `--errors-only` flag to filter ERROR level logs
  * Implement the `--search` flag to filter logs containing specific text
  * Add error handling for file not found and display a user-friendly message
  * Count and display the total number of each log level (INFO, WARNING, ERROR, DEBUG)

---

### **Part 3 – Structured Output & Data Analysis (50 min)**

**Building Block**: Presenting data in multiple formats

* **Concepts**:

  * Serialization with `serde`
  * Creating formatted tables with `prettytable-rs`
  * Writing CSV output
  * Pattern analysis (most common errors, time-based trends)
* **Demo Code**: Multi-format output

```rust
use prettytable::{Cell, Row, Table};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
struct LogStats {
    total_entries: usize,
    by_level: HashMap<String, usize>,
    top_errors: Vec<ErrorFrequency>,
}

#[derive(Debug, Serialize)]
struct ErrorFrequency {
    message: String,
    count: usize,
}

fn analyze_logs(entries: &[LogEntry]) -> LogStats {
    let mut by_level = HashMap::new();
    let mut error_messages: HashMap<String, usize> = HashMap::new();

    for entry in entries {
        let level_name = format!("{:?}", entry.level);
        *by_level.entry(level_name).or_insert(0) += 1;

        if entry.level == LogLevel::Error {
            *error_messages.entry(entry.message.clone()).or_insert(0) += 1;
        }
    }

    let mut top_errors: Vec<_> = error_messages
        .into_iter()
        .map(|(message, count)| ErrorFrequency { message, count })
        .collect();

    top_errors.sort_by(|a, b| b.count.cmp(&a.count));
    top_errors.truncate(5); // Top 5 errors

    LogStats {
        total_entries: entries.len(),
        by_level,
        top_errors,
    }
}

fn output_text(stats: &LogStats) {
    println!("\n Log Analysis Results");
    println!("========================\n");

    println!("Total entries: {}\n", stats.total_entries);

    println!("Breakdown by level:");
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("Level"),
        Cell::new("Count"),
        Cell::new("Percentage"),
    ]));

    for (level, count) in &stats.by_level {
        let percentage = (*count as f64 / stats.total_entries as f64) * 100.0;
        table.add_row(Row::new(vec![
            Cell::new(level),
            Cell::new(&count.to_string()),
            Cell::new(&format!("{:.1}%", percentage)),
        ]));
    }

    table.printstd();

    if !stats.top_errors.is_empty() {
        println!("\nTop errors:");
        let mut error_table = Table::new();
        error_table.add_row(Row::new(vec![
            Cell::new("Error Message"),
            Cell::new("Occurrences"),
        ]));

        for error in &stats.top_errors {
            error_table.add_row(Row::new(vec![
                Cell::new(&error.message),
                Cell::new(&error.count.to_string()),
            ]));
        }

        error_table.printstd();
    }
}

fn output_json(stats: &LogStats) {
    let json = serde_json::to_string_pretty(stats).unwrap();
    println!("{}", json);
}

fn output_csv(stats: &LogStats) {
    println!("level,count");
    for (level, count) in &stats.by_level {
        println!("{},{}", level, count);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let entries = read_logs(&cli.input)?;

    let filtered_entries: Vec<_> = entries.into_iter()
        .filter(|e| !cli.errors_only || e.level == LogLevel::Error)
        .collect();

    let stats = analyze_logs(&filtered_entries);

    match cli.format {
        OutputFormat::Text => output_text(&stats),
        OutputFormat::Json => output_json(&stats),
        OutputFormat::Csv => output_csv(&stats),
    }

    Ok(())
}
```

* **Exercise**:
  * Implement the `--top N` flag to show top N errors instead of hardcoded 5
  * Add time-based analysis: group errors by hour of day
  * In CSV output, include all statistics (not just by_level)
  * Add a `--output FILE` flag to write results to a file instead of stdout
  * Color-code the table output (red for ERROR, yellow for WARNING) using `colored` crate

---

### **Part 4 – Performance Optimization with Parallel Processing (40 min)**

**Building Block**: Scaling to large files

* **Concepts**:

  * Why parallel processing? (multi-core utilization)
  * Using `rayon` for data parallelism
  * When to parallelize (large files only)
  * Benchmarking performance improvements
* **Demo Code**: Parallel log processing

```rust
use rayon::prelude::*;
use std::time::Instant;

fn read_logs_parallel(path: &Path) -> Result<Vec<LogEntry>, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read all lines first
    let lines: Vec<_> = reader.lines().collect::<Result<_, _>>()?;

    // Parse in parallel
    let entries: Vec<_> = lines
        .par_iter()
        .filter_map(|line| parse_log_line(line))
        .collect();

    Ok(entries)
}

fn analyze_logs_parallel(entries: &[LogEntry]) -> LogStats {
    use std::sync::Mutex;

    let by_level = Mutex::new(HashMap::new());
    let error_messages = Mutex::new(HashMap::new());

    entries.par_iter().for_each(|entry| {
        let level_name = format!("{:?}", entry.level);
        *by_level.lock().unwrap()
            .entry(level_name)
            .or_insert(0) += 1;

        if entry.level == LogLevel::Error {
            *error_messages.lock().unwrap()
                .entry(entry.message.clone())
                .or_insert(0) += 1;
        }
    });

    let by_level = by_level.into_inner().unwrap();
    let error_messages = error_messages.into_inner().unwrap();

    let mut top_errors: Vec<_> = error_messages
        .into_iter()
        .map(|(message, count)| ErrorFrequency { message, count })
        .collect();

    top_errors.sort_by(|a, b| b.count.cmp(&a.count));
    top_errors.truncate(5);

    LogStats {
        total_entries: entries.len(),
        by_level,
        top_errors,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let start = Instant::now();

    // Use parallel version for files larger than 10MB
    let file_size = std::fs::metadata(&cli.input)?.len();
    let entries = if file_size > 10_000_000 {
        if cli.verbose {
            println!("Using parallel processing for large file...");
        }
        read_logs_parallel(&cli.input)?
    } else {
        read_logs(&cli.input)?
    };

    let parse_time = start.elapsed();

    let filtered_entries: Vec<_> = entries.into_iter()
        .filter(|e| !cli.errors_only || e.level == LogLevel::Error)
        .collect();

    let stats = analyze_logs_parallel(&filtered_entries);

    let total_time = start.elapsed();

    match cli.format {
        OutputFormat::Text => output_text(&stats),
        OutputFormat::Json => output_json(&stats),
        OutputFormat::Csv => output_csv(&stats),
    }

    if cli.verbose {
        eprintln!("\n⏱️  Performance:");
        eprintln!("  Parse time: {:?}", parse_time);
        eprintln!("  Total time: {:?}", total_time);
    }

    Ok(())
}
```

* **Exercise**:
  * Create a large test file (generate 100k log entries programmatically)
  * Benchmark sequential vs parallel processing
  * Add a `--parallel` flag to force parallel processing regardless of file size
  * Optimize the regex compilation (hint: use `lazy_static` or `once_cell`)
  * Add progress indicator for large files using `indicatif` crate

---

## Final Deliverable

Students will have a complete, production-ready CLI tool with:

- **Professional CLI**: Help text, argument validation, multiple output formats
- **Efficient parsing**: Regex-based log parsing, handles large files
- **Rich analysis**: Statistics by log level, top errors, pattern detection
- **Multiple output formats**: Text (with tables), JSON, CSV
- **Performance optimization**: Parallel processing for large files
- **Production quality**: Error handling, exit codes, verbose mode

### Usage Examples

```bash
# Basic analysis
./loglyzer application.log

# Show only errors in JSON format
./loglyzer --errors-only --format json app.log

# Get top 10 most common errors
./loglyzer --top 10 --errors-only app.log

# Search for specific pattern
./loglyzer --search "database" --format csv app.log > db_errors.csv

# Verbose mode with timing
./loglyzer --verbose large.log

# Help
./loglyzer --help
```

### Sample Output

```
 Log Analysis Results
========================

Total entries: 10

Breakdown by level:
+----------+-------+------------+
| Level    | Count | Percentage |
+----------+-------+------------+
| Info     | 6     | 60.0%      |
| Error    | 2     | 20.0%      |
| Warning  | 2     | 20.0%      |
+----------+-------+------------+

Top errors:
+--------------------------------------+-------------+
| Error Message                        | Occurrences |
+--------------------------------------+-------------+
| Failed to connect to API: timeout    | 1           |
| Database query failed: syntax error  | 1           |
+--------------------------------------+-------------+
```

---

## Extensions (Bonus Challenges)

1. **Real-time monitoring**: Use `--follow` flag (like `tail -f`) to watch logs in real-time
2. **Advanced patterns**: Extract IP addresses, URLs, error codes using regex groups
3. **Time-based filtering**: `--since "2024-01-15 10:00"` and `--until` flags
4. **Multiple file support**: Accept glob patterns (e.g., `*.log`)
5. **Web interface**: Add `--serve` flag to launch a web dashboard showing results
6. **Export formats**: Add HTML output with charts
7. **Configuration file**: Support `.loglyzer.toml` for default settings
8. **Plugin system**: Allow custom log format parsers

---

## Key Takeaways

1. **CLI Design**: Good UX, helpful errors, comprehensive help text
2. **File I/O**: Efficient reading with `BufReader`, line-by-line processing
3. **Pattern Matching**: Regex for structured data extraction
4. **Data Structures**: Choosing the right collections (HashMap, Vec)
5. **Serialization**: `serde` for multiple output formats
6. **Performance**: When and how to parallelize with `rayon`
7. **Error Handling**: Result types, proper error messages, exit codes

---

## Resources

- [clap Documentation](https://docs.rs/clap)
- [regex Documentation](https://docs.rs/regex)
- [serde Documentation](https://serde.rs)
- [rayon Documentation](https://docs.rs/rayon)
- [Rust CLI Book](https://rust-cli.github.io/book/)
- [Command Line Applications in Rust](https://rust-cli.github.io/book/index.html)

```

```
