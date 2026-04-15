//! Unified CLI output: colors and prefixes via [`console`] (respects `NO_COLOR`, non-TTY).

use console::Style;
use std::fmt::Display;

/// Theme for [`dialoguer`] prompts (`freeze`, etc.).
pub fn dialoguer_theme() -> dialoguer::theme::ColorfulTheme {
    dialoguer::theme::ColorfulTheme::default()
}

fn accent() -> Style {
    Style::new().cyan().bold()
}

fn good() -> Style {
    Style::new().green().bold()
}

fn bad() -> Style {
    Style::new().red().bold()
}

fn caution() -> Style {
    Style::new().yellow().bold()
}

fn subtle() -> Style {
    Style::new().dim()
}

/// Primary human-facing line (emphasis / next step).
pub fn status(msg: impl Display) {
    println!("{} {}", accent().apply_to("›"), msg);
}

/// Completed action.
pub fn success(msg: impl Display) {
    println!("{} {}", good().apply_to("✓"), msg);
}

pub fn warning(msg: impl Display) {
    println!("{} {}", caution().apply_to("!"), msg);
}

/// Problem line on stderr (diagnostics).
pub fn error_line(msg: impl Display) {
    eprintln!("{} {}", bad().apply_to("✗"), msg);
}

pub fn note(msg: impl Display) {
    println!("{} {}", subtle().apply_to("·"), msg);
}

/// In-progress detail (URL, sub-step).
pub fn step(msg: impl Display) {
    println!("{} {}", subtle().apply_to("…"), msg);
}

/// Unstyled line (TOML dumps, single path for piping).
pub fn raw(msg: impl Display) {
    println!("{msg}");
}

pub fn blank() {
    println!();
}

/// Section title for multi-step flows (e.g. NuGet resolve).
pub fn section_title(title: impl Display) {
    blank();
    println!("{}", accent().apply_to(format!("▸ {title}")));
}

pub fn labeled(label: &str, value: impl Display) {
    println!(
        "{} {}",
        Style::new().bold().apply_to(format!("{label}:")),
        value
    );
}

pub fn item_indent(msg: impl Display) {
    println!("  {msg}");
}

pub fn item_indent_dim(msg: impl Display) {
    println!("{}", subtle().apply_to(format!("  {msg}")));
}

/// Manifest list row: padded name, version, dim kind.
pub fn manifest_row(name: &str, version: &str, kind: &str, name_width: usize) {
    println!(
        "{:<name_width$}  {}  {}",
        name,
        version,
        subtle().apply_to(format!("({kind})")),
        name_width = name_width
    );
}
