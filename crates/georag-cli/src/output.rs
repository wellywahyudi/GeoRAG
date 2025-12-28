use console::style;
use serde::Serialize;
use std::fmt::Display;
use tabled::{settings::Style, Table, Tabled};

/// Output format mode
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Human,
    Json,
}

pub struct OutputWriter {
    format: OutputFormat,
}

impl OutputWriter {
    pub fn new(json: bool) -> Self {
        Self {
            format: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Human
            },
        }
    }

    pub fn success(&self, message: impl Display) {
        match self.format {
            OutputFormat::Human => {
                println!("{} {}", style("✓").green().bold(), message);
            }
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "status": "success",
                    "message": message.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
        }
    }

    pub fn info(&self, message: impl Display) {
        match self.format {
            OutputFormat::Human => {
                println!("{} {}", style("ℹ").blue().bold(), message);
            }
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "status": "info",
                    "message": message.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
        }
    }

    pub fn warning(&self, message: impl Display) {
        match self.format {
            OutputFormat::Human => {
                eprintln!("{} {}", style("⚠").yellow().bold(), message);
            }
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "status": "warning",
                    "message": message.to_string(),
                });
                eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
        }
    }
    pub fn error(&self, message: impl Display) {
        match self.format {
            OutputFormat::Human => {
                eprintln!("{} {}", style("✗").red().bold(), message);
            }
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "status": "error",
                    "message": message.to_string(),
                });
                eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
        }
    }

    pub fn table<T: Tabled>(&self, data: Vec<T>) {
        match self.format {
            OutputFormat::Human => {
                if data.is_empty() {
                    println!("{}", style("(no data)").dim());
                } else {
                    let mut table = Table::new(data);
                    table.with(Style::rounded());
                    println!("{}", table);
                }
            }
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "data": "table data"
                    }))
                    .unwrap()
                );
            }
        }
    }

    pub fn data<T: Serialize>(&self, data: &T) -> anyhow::Result<()> {
        let json_str = serde_json::to_string_pretty(data)?;
        println!("{}", json_str);
        Ok(())
    }

    pub fn result<T: Serialize>(&self, data: T) -> anyhow::Result<()> {
        match self.format {
            OutputFormat::Human => {
                self.data(&data)?;
            }
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "status": "success",
                    "data": data,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
        }
        Ok(())
    }

    pub fn kv(&self, key: impl Display, value: impl Display) {
        match self.format {
            OutputFormat::Human => {
                println!("{}: {}", style(key).bold(), value);
            }
            OutputFormat::Json => {
                let output = serde_json::json!({
                    key.to_string(): value.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
        }
    }

    pub fn section(&self, title: impl Display) {
        match self.format {
            OutputFormat::Human => {
                println!("\n{}", style(title).bold().underlined());
            }
            OutputFormat::Json => {}
        }
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }
}
