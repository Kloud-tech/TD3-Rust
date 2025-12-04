use chrono::NaiveDateTime;
use clap::{ArgAction, Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use prettytable::{Cell, Row, Table};
use rayon::prelude::*;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

static LOG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})\s+\[(\w+)\]\s+(.+)$").unwrap()
});

static LEVEL_COLOR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)\b(ERROR|WARNING)\b").unwrap());

const PARALLEL_THRESHOLD: u64 = 10 * 1024 * 1024; // 10 MB
const PROGRESS_THRESHOLD: u64 = 5 * 1024 * 1024; // 5 MB

#[derive(Debug, Parser)]
#[command(name = "loglyzer", about = "Analyse et filtre des fichiers de logs")]
struct Cli {
    /// Fichier de log à analyser
    #[arg(value_name = "LOG_FILE")]
    input: PathBuf,

    /// Ne garder que les entrées de niveau ERROR
    #[arg(long, action = ArgAction::SetTrue)]
    errors_only: bool,

    /// Texte à rechercher dans chaque entrée
    #[arg(long, value_name = "TEXT")]
    search: Option<String>,

    /// Nombre d'erreurs les plus fréquentes à afficher
    #[arg(long, value_name = "N", default_value_t = 5, value_parser = parse_top)]
    top: usize,

    /// Filtrer les logs à partir d'une date/heure (YYYY-MM-DD HH:MM:SS)
    #[arg(long, value_name = "DATETIME", value_parser = parse_datetime)]
    since: Option<NaiveDateTime>,

    /// Filtrer les logs jusqu'à une date/heure (YYYY-MM-DD HH:MM:SS)
    #[arg(long, value_name = "DATETIME", value_parser = parse_datetime)]
    until: Option<NaiveDateTime>,

    /// Format de sortie (text, json, csv)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Écrit le résultat dans un fichier au lieu de stdout
    #[arg(long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Force le mode parallèle quel que soit la taille du fichier
    #[arg(long, action = ArgAction::SetTrue)]
    parallel: bool,

    /// Affiche des informations de performance
    #[arg(long, action = ArgAction::SetTrue)]
    verbose: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Csv,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
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

    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
        }
    }
}

#[derive(Debug, Clone)]
struct LogEntry {
    timestamp: String,
    datetime: NaiveDateTime,
    level: LogLevel,
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorFrequency {
    message: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct LogStats {
    total_entries: usize,
    by_level: HashMap<String, usize>,
    top_errors: Vec<ErrorFrequency>,
    errors_by_hour: HashMap<String, usize>,
    error_rate_by_hour: HashMap<String, f64>,
    since: Option<String>,
    until: Option<String>,
    skipped_lines: usize,
}

#[derive(Debug)]
struct ParsedLogs {
    entries: Vec<LogEntry>,
    skipped: usize,
}

fn parse_log_line(line: &str) -> Option<LogEntry> {
    LOG_RE.captures(line).and_then(|caps| {
        let ts = caps.get(1)?.as_str();
        let datetime = NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S").ok()?;
        Some(LogEntry {
            timestamp: ts.to_string(),
            datetime,
            level: LogLevel::from_str(caps.get(2)?.as_str())?,
            message: caps.get(3)?.as_str().to_string(),
        })
    })
}

fn read_logs(path: &Path, pb: Option<&ProgressBar>) -> Result<ParsedLogs, std::io::Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buf = String::new();
    let mut entries = Vec::new();
    let mut skipped = 0usize;

    while reader.read_line(&mut buf)? != 0 {
        if let Some(entry) = parse_log_line(buf.trim_end_matches(['\n', '\r'])) {
            entries.push(entry);
        } else {
            skipped += 1;
        }
        if let Some(bar) = pb {
            bar.inc(buf.len() as u64);
        }
        buf.clear();
    }

    if let Some(bar) = pb {
        bar.finish_and_clear();
    }

    Ok(ParsedLogs { entries, skipped })
}

fn read_logs_parallel(path: &Path, pb: Option<&ProgressBar>) -> Result<ParsedLogs, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines = Vec::new();
    let mut skipped = 0usize;
    for line in reader.lines() {
        let line = line?;
        if let Some(bar) = pb {
            bar.inc(line.len() as u64 + 1);
        }
        lines.push(line);
    }

    if let Some(bar) = pb {
        bar.finish_and_clear();
    }

    let entries: Vec<_> = lines
        .par_iter()
        .filter_map(|line| parse_log_line(line))
        .collect();
    skipped += lines.len().saturating_sub(entries.len());

    Ok(ParsedLogs { entries, skipped })
}

fn analyze_logs(
    entries: &[LogEntry],
    top_n: usize,
    since: Option<NaiveDateTime>,
    until: Option<NaiveDateTime>,
    skipped: usize,
) -> LogStats {
    let mut by_level = HashMap::new();
    let mut error_messages = HashMap::new();
    let mut errors_by_hour = HashMap::new();

    for entry in entries {
        let level_name = entry.level.as_str().to_string();
        *by_level.entry(level_name.clone()).or_insert(0) += 1;

        if entry.level == LogLevel::Error {
            *error_messages.entry(entry.message.clone()).or_insert(0) += 1;
            if let Some(hour) = extract_hour(&entry.timestamp) {
                *errors_by_hour.entry(hour).or_insert(0) += 1;
            }
        }
    }

    let mut top_errors: Vec<_> = error_messages
        .into_iter()
        .map(|(message, count)| ErrorFrequency { message, count })
        .collect();

    top_errors.sort_by(|a, b| b.count.cmp(&a.count));
    top_errors.truncate(top_n.max(1));

    let error_rate_by_hour = if entries.is_empty() {
        HashMap::new()
    } else {
        errors_by_hour
            .iter()
            .map(|(k, v)| (k.clone(), (*v as f64 / entries.len() as f64) * 100.0))
            .collect()
    };

    LogStats {
        total_entries: entries.len(),
        by_level,
        top_errors,
        errors_by_hour,
        error_rate_by_hour,
        since: since.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()),
        until: until.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()),
        skipped_lines: skipped,
    }
}

fn extract_hour(ts: &str) -> Option<String> {
    let mut parts = ts.split_whitespace();
    let _date = parts.next()?;
    let time = parts.next()?;
    let hour = time.split(':').next()?;
    Some(format!("{hour}:00"))
}

fn render_text(stats: &LogStats, top_n: usize) -> String {
    use std::fmt::Write;

    let mut output = String::new();
    writeln!(output, "\n Log Analysis Results").unwrap();
    writeln!(output, "========================\n").unwrap();
    writeln!(output, "Total entries: {}\n", stats.total_entries).unwrap();
    if stats.skipped_lines > 0 {
        writeln!(
            output,
            "Lignes ignorées (format invalide): {}\n",
            stats.skipped_lines
        )
        .unwrap();
    }

    if stats.since.is_some() || stats.until.is_some() {
        writeln!(output, "Filtres appliqués:").unwrap();
        if let Some(s) = &stats.since {
            writeln!(output, "- Depuis : {s}").unwrap();
        }
        if let Some(u) = &stats.until {
            writeln!(output, "- Jusqu'à : {u}").unwrap();
        }
        writeln!(output).unwrap();
    }

    writeln!(output, "Breakdown by level:").unwrap();
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("Level"),
        Cell::new("Count"),
        Cell::new("Percentage"),
    ]));

    let mut levels: Vec<_> = stats.by_level.iter().collect();
    levels.sort_by(|a, b| a.0.cmp(b.0));

    for (level, count) in levels {
        let percentage = if stats.total_entries > 0 {
            (*count as f64 / stats.total_entries as f64) * 100.0
        } else {
            0.0
        };
        table.add_row(Row::new(vec![
            Cell::new(level),
            Cell::new(&count.to_string()),
            Cell::new(&format!("{:.1}%", percentage)),
        ]));
    }
    let table_str = table.to_string();
    let table_str = colorize_levels(&table_str);
    writeln!(output, "{table_str}").unwrap();

    if !stats.top_errors.is_empty() {
        writeln!(output, "\nTop errors (max {top_n}):").unwrap();
        let mut error_table = Table::new();
        error_table.add_row(Row::new(vec![
            Cell::new("Error Message"),
            Cell::new("Occurrences"),
        ]));

        for err in &stats.top_errors {
            error_table.add_row(Row::new(vec![
                Cell::new(&err.message),
                Cell::new(&err.count.to_string()),
            ]));
        }

        writeln!(output, "{error_table}").unwrap();
    }

    if !stats.errors_by_hour.is_empty() {
        writeln!(output, "\nErrors by hour:").unwrap();
        let mut hour_table = Table::new();
        hour_table.add_row(Row::new(vec![Cell::new("Hour"), Cell::new("Count")]));

        let mut hours: Vec<_> = stats.errors_by_hour.iter().collect();
        hours.sort_by(|a, b| a.0.cmp(b.0));

        for (hour, count) in hours {
            hour_table.add_row(Row::new(vec![
                Cell::new(hour),
                Cell::new(&count.to_string()),
            ]));
        }

        writeln!(output, "{hour_table}").unwrap();
    }

    if !stats.error_rate_by_hour.is_empty() {
        writeln!(output, "\nError rate by hour:").unwrap();
        let mut rate_table = Table::new();
        rate_table.add_row(Row::new(vec![Cell::new("Hour"), Cell::new("Error %")]));

        let mut hours: Vec<_> = stats.error_rate_by_hour.iter().collect();
        hours.sort_by(|a, b| a.0.cmp(b.0));

        for (hour, rate) in hours {
            rate_table.add_row(Row::new(vec![
                Cell::new(hour),
                Cell::new(&format!("{:.2}%", rate)),
            ]));
        }

        writeln!(output, "{rate_table}").unwrap();
    }

    output
}

fn render_json(stats: &LogStats) -> String {
    serde_json::to_string_pretty(stats).unwrap_or_else(|_| "{}".to_string())
}

fn render_csv(stats: &LogStats) -> String {
    let mut output = String::from("metric,key,value\n");
    output.push_str(&format!("total,,{}\n", stats.total_entries));
    if stats.skipped_lines > 0 {
        output.push_str(&format!("skipped,,{}\n", stats.skipped_lines));
    }
    if let Some(s) = &stats.since {
        output.push_str(&format!("filter,since,{s}\n"));
    }
    if let Some(u) = &stats.until {
        output.push_str(&format!("filter,until,{u}\n"));
    }

    let mut levels: Vec<_> = stats.by_level.iter().collect();
    levels.sort_by(|a, b| a.0.cmp(b.0));
    for (level, count) in levels {
        output.push_str(&format!("level,{level},{count}\n"));
    }

    for err in &stats.top_errors {
        let msg = err.message.replace('"', "\"\"");
        output.push_str(&format!("top_error,\"{msg}\",{}\n", err.count));
    }

    let mut hours: Vec<_> = stats.errors_by_hour.iter().collect();
    hours.sort_by(|a, b| a.0.cmp(b.0));
    for (hour, count) in hours {
        output.push_str(&format!("error_by_hour,{hour},{count}\n"));
    }

    let mut rates: Vec<_> = stats.error_rate_by_hour.iter().collect();
    rates.sort_by(|a, b| a.0.cmp(b.0));
    for (hour, rate) in rates {
        output.push_str(&format!("error_rate_by_hour,{hour},{:.4}\n", rate));
    }

    output
}

fn colorize_levels(table: &str) -> String {
    use colored::Colorize;

    LEVEL_COLOR_RE
        .replace_all(table, |caps: &regex::Captures<'_>| match &caps[1] {
            "ERROR" => "ERROR".red().bold().to_string(),
            "WARNING" => "WARNING".yellow().bold().to_string(),
            other => other.to_string(),
        })
        .to_string()
}

fn make_progress_bar(size: u64) -> ProgressBar {
    let pb = ProgressBar::new(size);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})",
        )
        .unwrap()
        .progress_chars("=>-"),
    );
    pb
}

fn should_use_progress(size: u64) -> bool {
    size >= PROGRESS_THRESHOLD
}

fn parse_datetime(input: &str) -> Result<NaiveDateTime, String> {
    NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| format!("Format attendu: YYYY-MM-DD HH:MM:SS ({e})"))
}

fn parse_top(input: &str) -> Result<usize, String> {
    let value: usize = input
        .parse()
        .map_err(|_| "La valeur de --top doit être un entier positif".to_string())?;
    if value == 0 {
        Err("La valeur de --top doit être au moins 1".to_string())
    } else {
        Ok(value)
    }
}

fn filter_entries(
    entries: Vec<LogEntry>,
    errors_only: bool,
    search_lower: Option<&str>,
    since: Option<NaiveDateTime>,
    until: Option<NaiveDateTime>,
) -> Vec<LogEntry> {
    entries
        .into_iter()
        .filter(|e| !errors_only || e.level == LogLevel::Error)
        .filter(|e| {
            if let Some(since) = since {
                e.datetime >= since
            } else {
                true
            }
        })
        .filter(|e| {
            if let Some(until) = until {
                e.datetime <= until
            } else {
                true
            }
        })
        .filter(|e| {
            if let Some(term) = search_lower {
                let haystack =
                    format!("{} [{}] {}", e.timestamp, e.level.as_str(), e.message).to_lowercase();
                haystack.contains(term)
            } else {
                true
            }
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let top_n = cli.top.max(1);

    let meta = match fs::metadata(&cli.input) {
        Ok(m) => m,
        Err(err) => {
            use std::io::ErrorKind;
            match err.kind() {
                ErrorKind::NotFound => {
                    eprintln!("Fichier introuvable: {}", cli.input.display());
                    std::process::exit(2);
                }
                _ => {
                    eprintln!(
                        "Impossible de lire le fichier {}: {}",
                        cli.input.display(),
                        err
                    );
                    std::process::exit(1);
                }
            }
        }
    };
    let file_size = meta.len();

    let use_parallel = cli.parallel || file_size > PARALLEL_THRESHOLD;
    let start = Instant::now();

    if cli.verbose {
        eprintln!(
            "Lecture de {} ({} octets) en mode {}",
            cli.input.display(),
            file_size,
            if use_parallel {
                "parallèle"
            } else {
                "séquentiel"
            }
        );
    }

    let progress = if should_use_progress(file_size) {
        Some(make_progress_bar(file_size))
    } else {
        None
    };

    let parsed = if use_parallel {
        read_logs_parallel(&cli.input, progress.as_ref())
    } else {
        read_logs(&cli.input, progress.as_ref())
    };

    let parsed = match parsed {
        Ok(list) => list,
        Err(err) => {
            use std::io::ErrorKind;
            match err.kind() {
                ErrorKind::NotFound => {
                    eprintln!("Fichier introuvable: {}", cli.input.display());
                    std::process::exit(2);
                }
                _ => return Err(Box::new(err)),
            }
        }
    };

    let parse_time = start.elapsed();

    let search_lower = cli.search.as_ref().map(|s| s.to_lowercase());
    let filtered = filter_entries(
        parsed.entries,
        cli.errors_only,
        search_lower.as_deref(),
        cli.since,
        cli.until,
    );

    if filtered.is_empty() {
        let msg = "Aucune entrée ne correspond aux filtres fournis.";
        if let Some(path) = cli.output {
            fs::write(&path, msg)?;
            println!("Résultats écrits dans {}", path.display());
        } else {
            println!("{msg}");
        }
        return Ok(());
    }

    let stats = analyze_logs(&filtered, top_n, cli.since, cli.until, parsed.skipped);
    let analysis_time = start.elapsed() - parse_time;

    let rendered = match cli.format {
        OutputFormat::Text => render_text(&stats, top_n),
        OutputFormat::Json => render_json(&stats),
        OutputFormat::Csv => render_csv(&stats),
    };

    if let Some(path) = cli.output {
        fs::write(&path, rendered)?;
        println!("Résultats écrits dans {}", path.display());
    } else {
        println!("{rendered}");
    }

    if cli.verbose {
        let total_time = start.elapsed();
        eprintln!(
            "\n⏱️  Performance: parse={:?}, analyse={:?}, total={:?}",
            parse_time, analysis_time, total_time
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn entry(line: &str) -> LogEntry {
        parse_log_line(line).expect("log line should parse")
    }

    #[test]
    fn parse_log_line_parses_fields() {
        let line = "2024-01-15 10:30:45 [ERROR] Failed to connect";
        let e = entry(line);
        assert_eq!(e.timestamp, "2024-01-15 10:30:45");
        assert_eq!(e.level, LogLevel::Error);
        assert_eq!(e.message, "Failed to connect");
        assert_eq!(
            e.datetime,
            NaiveDateTime::parse_from_str("2024-01-15 10:30:45", "%Y-%m-%d %H:%M:%S").unwrap()
        );
    }

    #[test]
    fn parse_log_line_invalid_returns_none() {
        assert!(parse_log_line("not a log line").is_none());
        assert!(parse_log_line("2024-01-15 [INFO] missing time").is_none());
    }

    #[test]
    fn filter_entries_respects_flags() {
        let entries = vec![
            entry("2024-01-15 10:30:45 [ERROR] API timeout"),
            entry("2024-01-15 10:31:45 [INFO] OK"),
            entry("2024-01-15 10:32:45 [ERROR] Database down"),
        ];

        let since = parse_datetime("2024-01-15 10:30:00").ok();
        let filtered = filter_entries(entries, true, Some("api"), since, None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].message, "API timeout");
    }

    #[test]
    fn analyze_logs_counts_levels_and_top() {
        let entries = vec![
            entry("2024-01-15 10:30:45 [ERROR] API timeout"),
            entry("2024-01-15 10:31:45 [ERROR] API timeout"),
            entry("2024-01-15 10:32:45 [INFO] OK"),
            entry("2024-01-15 10:33:45 [WARNING] High CPU"),
        ];

        let stats = analyze_logs(&entries, 3, None, None, 0);
        assert_eq!(stats.total_entries, 4);
        assert_eq!(stats.by_level.get("ERROR"), Some(&2));
        assert_eq!(stats.by_level.get("INFO"), Some(&1));
        assert_eq!(stats.by_level.get("WARNING"), Some(&1));
        assert_eq!(stats.top_errors.first().map(|e| e.count), Some(2));
    }
}
