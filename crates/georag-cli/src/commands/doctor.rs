use crate::auto_detect::{detect_workspace_issues, OllamaDetection, PostgresDetection};
use crate::cli::DoctorArgs;
use crate::config;
use crate::output::OutputWriter;
use anyhow::Result;
use console::style;

pub fn execute(args: DoctorArgs, _output: &OutputWriter) -> Result<()> {
    println!("\n{}", style("GeoRAG Health Check").bold().underlined());
    println!("{}", style("═".repeat(60)).dim());
    println!();

    let mut checks_passed = 0;
    let mut total_checks = 0;

    // Check workspace
    total_checks += 1;
    match config::find_workspace_root() {
        Ok(workspace_path) => {
            println!("{} Workspace: Found at {}", style("✓").green(), workspace_path.display());
            checks_passed += 1;

            // Check for issues
            let issues = detect_workspace_issues(&workspace_path);
            if !issues.is_empty() {
                for issue in issues {
                    println!("  {} {}", style("⚠").yellow(), issue);
                }
            }

            // Check config
            total_checks += 1;
            match config::load_config_with_fallback(&workspace_path) {
                Ok(config) => {
                    println!("{} Config: Valid configuration", style("✓").green());
                    checks_passed += 1;

                    if args.verbose {
                        println!("  Storage backend: {}", config.storage.backend);
                        if let Some(pg) = config.postgres {
                            println!("  PostgreSQL: {}:{}/{}", pg.host, pg.port, pg.database);
                        }
                    }
                }
                Err(e) => {
                    println!("{} Config: {}", style("✗").red(), e);
                }
            }

            // Check datasets
            total_checks += 1;
            let datasets_file = workspace_path.join(".georag").join("datasets.json");
            if datasets_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&datasets_file) {
                    if let Ok(datasets) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                        if datasets.is_empty() {
                            println!("{} Datasets: No datasets registered", style("⚠").yellow());
                            println!("  → Run: georag add dataset.geojson");
                        } else {
                            println!(
                                "{} Datasets: {} datasets registered",
                                style("✓").green(),
                                datasets.len()
                            );
                            checks_passed += 1;
                        }
                    }
                }
            } else {
                println!("{} Datasets: datasets.json not found", style("✗").red());
            }

            // Check index
            total_checks += 1;
            let index_file = workspace_path.join(".georag").join("index").join("state.json");
            if index_file.exists() {
                println!("{} Index: Built", style("✓").green());
                checks_passed += 1;
            } else {
                println!("{} Index: Not built", style("⚠").yellow());
                println!("  → Run: georag build");
            }
        }
        Err(_) => {
            println!("{} Workspace: Not found", style("✗").red());
            println!("  → Run: georag init");
        }
    }

    println!();
    println!("{}", style("PostgreSQL Check").bold());
    println!("{}", style("─".repeat(60)).dim());

    // Check PostgreSQL
    let pg_detection = PostgresDetection::detect();

    total_checks += 1;
    if pg_detection.installed {
        println!("{} PostgreSQL: Installed", style("✓").green());
        checks_passed += 1;

        if let Some(ref version) = pg_detection.version {
            if args.verbose {
                println!("  Version: {}", version);
            }
        }
    } else {
        println!("{} PostgreSQL: Not installed", style("✗").red());
        println!("  → Install: brew install postgresql (macOS)");
        println!("  → Or: apt-get install postgresql (Ubuntu)");
    }

    total_checks += 1;
    if pg_detection.running {
        println!("{} PostgreSQL: Running", style("✓").green());
        checks_passed += 1;

        if let Some(ref url) = pg_detection.suggested_url {
            if args.verbose {
                println!("  Suggested URL: {}", url);
            }
        }
    } else if pg_detection.installed {
        println!("{} PostgreSQL: Not running", style("⚠").yellow());
        println!("  → Start: brew services start postgresql (macOS)");
        println!("  → Or: sudo systemctl start postgresql (Linux)");
    }

    // Check DATABASE_URL
    total_checks += 1;
    if let Ok(url) = std::env::var("DATABASE_URL") {
        println!("{} DATABASE_URL: Set", style("✓").green());
        checks_passed += 1;

        if args.verbose {
            // Hide password
            let display_url = url.split('@').next_back().unwrap_or(&url);
            println!("  Value: ...@{}", display_url);
        }
    } else {
        println!("{} DATABASE_URL: Not set", style("⚠").yellow());
        println!("  → Set: export DATABASE_URL=\"postgresql://localhost/georag\"");
    }

    println!();
    println!("{}", style("Embedder Check").bold());
    println!("{}", style("─".repeat(60)).dim());

    // Check Ollama
    let ollama_detection = OllamaDetection::detect();

    total_checks += 1;
    if ollama_detection.installed {
        println!("{} Ollama: Installed", style("✓").green());
        checks_passed += 1;
    } else {
        println!("{} Ollama: Not installed", style("✗").red());
        println!("  → Install: https://ollama.ai/download");
    }

    total_checks += 1;
    if ollama_detection.running {
        println!("{} Ollama: Running", style("✓").green());
        checks_passed += 1;

        if args.verbose && !ollama_detection.available_models.is_empty() {
            println!("  Available models:");
            for model in &ollama_detection.available_models {
                println!("    • {}", model);
            }
        }
    } else if ollama_detection.installed {
        println!("{} Ollama: Not running", style("⚠").yellow());
        println!("  → Start: ollama serve");
    }

    // Check for nomic-embed-text model
    total_checks += 1;
    if ollama_detection.has_model("nomic-embed-text") {
        println!("{} Model: nomic-embed-text available", style("✓").green());
        checks_passed += 1;
    } else if ollama_detection.running {
        println!("{} Model: nomic-embed-text not found", style("⚠").yellow());
        println!("  → Pull: ollama pull nomic-embed-text");
    }

    // Summary
    println!();
    println!("{}", style("═".repeat(60)).dim());

    let percentage = (checks_passed as f64 / total_checks as f64 * 100.0) as usize;
    let status_icon = if percentage >= 80 {
        style("✓").green()
    } else if percentage >= 50 {
        style("⚠").yellow()
    } else {
        style("✗").red()
    };

    println!(
        "{} Overall Status: {}/{} checks passed ({}%)",
        status_icon, checks_passed, total_checks, percentage
    );
    println!();

    if checks_passed < total_checks {
        println!(
            "{}",
            style("Some issues were found. Follow the suggestions above to fix them.").yellow()
        );
    } else {
        println!("{}", style("All checks passed! Your GeoRAG installation is healthy.").green());
    }

    Ok(())
}
