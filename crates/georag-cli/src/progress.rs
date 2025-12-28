#![allow(dead_code)]

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

/// Create a spinner for indeterminate progress
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a progress bar for determinate progress
pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n[{bar:40.cyan/blue}] {pos}/{len} ({percent}%) ETA: {eta}")
            .unwrap()
            .progress_chars("█▓▒░ "),
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a multi-progress container for multiple progress bars
pub fn create_multi_progress() -> MultiProgress {
    MultiProgress::new()
}

/// Finish a progress bar with success message
pub fn finish_success(pb: &ProgressBar, message: &str) {
    pb.finish_with_message(format!("✓ {}", message));
}

/// Finish a progress bar with error message
pub fn finish_error(pb: &ProgressBar, message: &str) {
    pb.finish_with_message(format!("✗ {}", message));
}

/// Progress tracker for build operations
pub struct BuildProgress {
    pub multi: MultiProgress,
    pub load_datasets: ProgressBar,
    pub normalize: ProgressBar,
    pub validate: ProgressBar,
    pub embeddings: ProgressBar,
    pub finalize: ProgressBar,
}

impl BuildProgress {
    pub fn new() -> Self {
        let multi = create_multi_progress();

        let load_datasets = multi.add(create_spinner("Loading datasets..."));
        let normalize = multi.add(create_spinner("Normalizing geometries..."));
        let validate = multi.add(create_spinner("Validating geometries..."));
        let embeddings = multi.add(create_spinner("Generating embeddings..."));
        let finalize = multi.add(create_spinner("Building index..."));

        Self {
            multi,
            load_datasets,
            normalize,
            validate,
            embeddings,
            finalize,
        }
    }

    pub fn start_load_datasets(&self) {
        self.load_datasets.set_message("Loading datasets...");
    }

    pub fn finish_load_datasets(&self, count: usize, features: usize) {
        finish_success(
            &self.load_datasets,
            &format!("Loaded {} datasets with {} features", count, features),
        );
    }

    pub fn start_normalize(&self) {
        self.normalize.set_message("Normalizing geometries...");
    }

    pub fn finish_normalize(&self, count: usize) {
        if count > 0 {
            finish_success(&self.normalize, &format!("Normalized {} datasets", count));
        } else {
            finish_success(&self.normalize, "All datasets already in workspace CRS");
        }
    }

    pub fn start_validate(&self) {
        self.validate.set_message("Validating geometries...");
    }

    pub fn finish_validate(&self, fixed: usize) {
        finish_success(&self.validate, &format!("Fixed {} invalid geometries", fixed));
    }

    pub fn start_embeddings(&self, total: u64) {
        self.embeddings.finish_and_clear();
        let pb = self.multi.add(create_progress_bar(total, "Generating embeddings"));
        self.embeddings.set_position(0);
        self.embeddings.set_length(total);
        self.embeddings.set_style(pb.style().clone());
    }

    pub fn update_embeddings(&self, current: u64) {
        self.embeddings.set_position(current);
    }

    pub fn finish_embeddings(&self, count: usize) {
        finish_success(&self.embeddings, &format!("Generated {} embeddings", count));
    }

    pub fn start_finalize(&self) {
        self.finalize.set_message("Building index...");
    }

    pub fn finish_finalize(&self, hash: &str) {
        finish_success(&self.finalize, &format!("Index built (hash: {})", &hash[..8]));
    }
}

/// Progress tracker for migration operations
pub struct MigrationProgress {
    pub multi: MultiProgress,
    pub connect: ProgressBar,
    pub migrations: ProgressBar,
    pub datasets: ProgressBar,
    pub features: ProgressBar,
    pub chunks: ProgressBar,
    pub embeddings: ProgressBar,
    pub verify: ProgressBar,
}

impl MigrationProgress {
    pub fn new() -> Self {
        let multi = create_multi_progress();

        let connect = multi.add(create_spinner("Connecting to PostgreSQL..."));
        let migrations = multi.add(create_spinner("Running migrations..."));
        let datasets = multi.add(create_spinner("Migrating datasets..."));
        let features = multi.add(create_spinner("Migrating features..."));
        let chunks = multi.add(create_spinner("Migrating chunks..."));
        let embeddings = multi.add(create_spinner("Migrating embeddings..."));
        let verify = multi.add(create_spinner("Verifying data integrity..."));

        Self {
            multi,
            connect,
            migrations,
            datasets,
            features,
            chunks,
            embeddings,
            verify,
        }
    }

    pub fn start_connect(&self) {
        self.connect.set_message("Connecting to PostgreSQL...");
    }

    pub fn finish_connect(&self) {
        finish_success(&self.connect, "Connected to PostgreSQL");
    }

    pub fn start_migrations(&self) {
        self.migrations.set_message("Running migrations...");
    }

    pub fn finish_migrations(&self, count: usize) {
        finish_success(&self.migrations, &format!("Applied {} migrations", count));
    }

    pub fn start_datasets(&self, total: u64) {
        self.datasets.finish_and_clear();
        let pb = self.multi.add(create_progress_bar(total, "Migrating datasets"));
        self.datasets.set_position(0);
        self.datasets.set_length(total);
        self.datasets.set_style(pb.style().clone());
    }

    pub fn update_datasets(&self, current: u64) {
        self.datasets.set_position(current);
    }

    pub fn finish_datasets(&self, count: usize) {
        finish_success(&self.datasets, &format!("Migrated {} datasets", count));
    }

    pub fn start_features(&self, total: u64) {
        self.features.finish_and_clear();
        let pb = self.multi.add(create_progress_bar(total, "Migrating features"));
        self.features.set_position(0);
        self.features.set_length(total);
        self.features.set_style(pb.style().clone());
    }

    pub fn update_features(&self, current: u64) {
        self.features.set_position(current);
    }

    pub fn finish_features(&self, count: usize) {
        finish_success(&self.features, &format!("Migrated {} features", count));
    }

    pub fn start_chunks(&self, total: u64) {
        self.chunks.finish_and_clear();
        let pb = self.multi.add(create_progress_bar(total, "Migrating chunks"));
        self.chunks.set_position(0);
        self.chunks.set_length(total);
        self.chunks.set_style(pb.style().clone());
    }

    pub fn update_chunks(&self, current: u64) {
        self.chunks.set_position(current);
    }

    pub fn finish_chunks(&self, count: usize) {
        finish_success(&self.chunks, &format!("Migrated {} chunks", count));
    }

    pub fn start_embeddings(&self, total: u64) {
        self.embeddings.finish_and_clear();
        let pb = self.multi.add(create_progress_bar(total, "Migrating embeddings"));
        self.embeddings.set_position(0);
        self.embeddings.set_length(total);
        self.embeddings.set_style(pb.style().clone());
    }

    pub fn update_embeddings(&self, current: u64) {
        self.embeddings.set_position(current);
    }

    pub fn finish_embeddings(&self, count: usize) {
        finish_success(&self.embeddings, &format!("Migrated {} embeddings", count));
    }

    pub fn start_verify(&self) {
        self.verify.set_message("Verifying data integrity...");
    }

    pub fn finish_verify(&self, success: bool) {
        if success {
            finish_success(&self.verify, "Data integrity verified");
        } else {
            finish_error(&self.verify, "Data integrity check failed");
        }
    }
}
