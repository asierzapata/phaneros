//! Phaneros CLI UI Styleguide & Component Toolkit.
//!
//! Standardized interface for rendering CLI output, including formatted tables,
//! key-value summary cards, section headers, badges, and numeric units.

#![allow(dead_code)]

use std::fmt::Display;

/// Terminal display configuration and width detection.
pub fn get_terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(80)
}

/// Truncate a string to fit within a maximum character length, appending an ellipsis if needed.
pub fn truncate_to_width(s: &str, max_width: usize, ellipsis: &str) -> String {
    let char_count = s.chars().count();
    if char_count <= max_width {
        return s.to_string();
    }
    let ellipsis_len = ellipsis.chars().count();
    if max_width <= ellipsis_len {
        return s.chars().take(max_width).collect();
    }
    let keep = max_width - ellipsis_len;
    let mut truncated: String = s.chars().take(keep).collect();
    truncated.push_str(ellipsis);
    truncated
}

/// Format bytes into human-readable units (B, KB, MB, GB, TB).
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format transfer rate in bytes per second (e.g., "1.23 MB/s").
pub fn format_speed(bps: u64) -> String {
    format!("{}/s", format_bytes(bps))
}

/// Format an epoch timestamp (seconds) into a human-readable local date & time (e.g., "2026-08-08 20:00:35").
pub fn format_timestamp(epoch_secs: u64) -> String {
    if epoch_secs == 0 {
        return "-".to_string();
    }
    jiff::Timestamp::from_second(epoch_secs as i64)
        .map(|ts| {
            let zoned = ts.to_zoned(jiff::tz::TimeZone::system());
            zoned.strftime("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|_| epoch_secs.to_string())
}

/// Column alignment options for CLI tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Right,
}

/// Column configuration for tables.
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub align: Alignment,
}

impl Column {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            align: Alignment::Left,
        }
    }

    pub fn right_aligned(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            align: Alignment::Right,
        }
    }
}

/// Dynamic, dependency-free CLI table builder with auto-column width calculations.
#[derive(Debug, Default)]
pub struct Table {
    columns: Vec<Column>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(headers: &[&str]) -> Self {
        Self {
            columns: headers.iter().map(|h| Column::new(*h)).collect(),
            rows: Vec::new(),
        }
    }

    pub fn with_columns(columns: Vec<Column>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
        }
    }

    pub fn add_row<I, T>(&mut self, row: I)
    where
        I: IntoIterator<Item = T>,
        T: Display,
    {
        self.rows
            .push(row.into_iter().map(|item| item.to_string()).collect());
    }

    pub fn render(&self) -> String {
        if self.columns.is_empty() {
            return String::new();
        }

        let mut widths: Vec<usize> = self.columns.iter().map(|c| c.name.len()).collect();

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        let mut output = String::new();

        // Render Header
        let header_line: Vec<String> = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| match col.align {
                Alignment::Left => format!("{:<width$}", col.name, width = widths[i]),
                Alignment::Right => format!("{:>width$}", col.name, width = widths[i]),
            })
            .collect();
        output.push_str(&header_line.join("  "));
        output.push('\n');

        // Render Divider
        let divider: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
        output.push_str(&divider.join("  "));
        output.push('\n');

        // Render Rows
        for row in &self.rows {
            let row_line: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let align = self
                        .columns
                        .get(i)
                        .map(|c| c.align)
                        .unwrap_or(Alignment::Left);
                    let width = widths.get(i).copied().unwrap_or(cell.len());
                    match align {
                        Alignment::Left => format!("{:<width$}", cell, width = width),
                        Alignment::Right => format!("{:>width$}", cell, width = width),
                    }
                })
                .collect();
            output.push_str(&row_line.join("  "));
            output.push('\n');
        }

        output
    }

    pub fn print(&self) {
        print!("{}", self.render());
    }
}

/// Helper function to quickly render a table from headers and row slices.
pub fn render_table(headers: &[&str], rows: &[Vec<String>]) {
    let mut table = Table::new(headers);
    for row in rows {
        table.add_row(row.clone());
    }
    table.print();
}

/// Render a top-level view header (e.g. `▌ Drive Status`).
pub fn render_view_header(title: &str) {
    println!("\n▌ {}\n", title);
}

/// Render a section header divider.
pub fn render_section_header(title: &str) {
    println!("\n--- {} ---", title);
}

/// Render a key-value detail list with uniform key alignment.
pub fn render_key_values(items: &[(&str, String)]) {
    let max_key_len = items.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    for (key, val) in items {
        println!(
            "{:<width$}  {}",
            format!("{}:", key),
            val,
            width = max_key_len + 1
        );
    }
}
