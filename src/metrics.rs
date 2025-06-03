use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::fs::{OpenOptions, File};
use std::io::{self, BufRead, Write};
use uuid::Uuid;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MetricEvent {
    AppLaunched,
    OutputDirectorySet,
    AppAdded {
        app_name: String,
    },
    AppRemoved {
        app_name: String,
    },
    AppRenamed {
        old_app_name: String,
        new_app_name: String,
    },
    IpaGenerated {
        app_name: String,
        success: bool,
        duration_ms: u128,
        output_size_bytes: u64,
    },
    AppConfigEdited {
        app_id: String, // Using app_id to identify which config was edited
    },
    // Could add more like ThemeChanged, ConfigOpened etc.
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetricEntry {
    pub id: Uuid, // Unique ID for each metric entry
    pub timestamp: DateTime<Utc>,
    pub event: MetricEvent,
    pub country_code: Option<String>, // To be added later if possible
    pub sent_to_server: bool, // To track if this metric has been uploaded
}

impl MetricEntry {
    pub fn new(event: MetricEvent) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event,
            country_code: None, // Placeholder for now
            sent_to_server: false,
        }
    }
}

#[derive(Debug)] // No Serialize/Deserialize for the collector itself, path is runtime
pub struct MetricsCollector {
    metrics_file_path: PathBuf,
    pub metrics: Vec<MetricEntry>, // Made public to be accessed by app for calculations
}

impl MetricsCollector {
    pub fn new(file_path: PathBuf) -> Self {
        // Ensure the directory for the metrics file exists
        if let Some(parent_dir) = file_path.parent() {
            if !parent_dir.exists() {
                if let Err(e) = std::fs::create_dir_all(parent_dir) {
                    log::error!("Failed to create directory for metrics file {}: {}", parent_dir.display(), e);
                }
            }
        }
        let mut collector = Self { metrics_file_path: file_path, metrics: Vec::new() };
        collector.load_metrics_from_file();
        collector
    }

    fn load_metrics_from_file(&mut self) {
        if !self.metrics_file_path.exists() {
            return; // No file, no metrics
        }

        let file = File::open(&self.metrics_file_path).unwrap();
        for line_result in io::BufReader::new(file).lines() {
            let line = line_result.unwrap();
            if line.trim().is_empty() { continue; }
            match serde_json::from_str::<MetricEntry>(&line) {
                Ok(entry) => {
                    self.metrics.push(entry);
                }
                Err(e) => {
                    log::warn!("Failed to parse metric line '{}': {}", line, e);
                }
            }
        }
    }

    pub fn record(&mut self, event: MetricEvent) {
        let entry = MetricEntry::new(event);
        self.metrics.push(entry.clone());
        match serde_json::to_string(&entry) {
            Ok(json_string) => {
                match OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.metrics_file_path) {
                    Ok(mut file) => {
                        if let Err(e) = writeln!(file, "{}", json_string) {
                            log::error!("Failed to write metric to {}: {}", self.metrics_file_path.display(), e);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to open metrics file {}: {}", self.metrics_file_path.display(), e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to serialize metric entry: {}", e);
            }
        }
    }

    #[allow(dead_code)]
    pub fn load_unsent_metrics(&self) -> io::Result<Vec<MetricEntry>> {
        let mut unsent_metrics = Vec::new();
        for entry in &self.metrics {
            if !entry.sent_to_server {
                unsent_metrics.push(entry.clone());
            }
        }
        Ok(unsent_metrics)
    }

    #[allow(dead_code)]
    pub fn mark_metrics_as_sent(&self, sent_ids: &[Uuid]) -> io::Result<()> {
        if self.metrics_file_path.exists() && !sent_ids.is_empty() {
            let temp_file_path = self.metrics_file_path.with_extension("jsonl.tmp");
            
            let mut writer = io::BufWriter::new(File::create(&temp_file_path)?);

            for entry in &self.metrics {
                let mut updated_entry = entry.clone();
                if sent_ids.contains(&entry.id) {
                    updated_entry.sent_to_server = true;
                }
                let updated_line = serde_json::to_string(&updated_entry).unwrap_or_else(|_| serde_json::to_string(entry).unwrap());
                writeln!(writer, "{}", updated_line)?;
            }
            writer.flush()?;
            drop(writer); // Ensure file is closed before rename

            std::fs::rename(&temp_file_path, &self.metrics_file_path)?;
        }
        Ok(())
    }

    // Methods for dashboard statistics
    pub fn generations_today(&self) -> usize {
        let today = Utc::now().date_naive();
        self.metrics.iter().filter(|entry| {
            if let MetricEvent::IpaGenerated { success, .. } = &entry.event {
                *success && entry.timestamp.date_naive() == today
            } else {
                false
            }
        }).count()
    }

    pub fn generations_all_time(&self) -> usize {
        self.metrics.iter().filter(|entry| {
            if let MetricEvent::IpaGenerated { success, .. } = &entry.event {
                *success
            } else {
                false
            }
        }).count()
    }

    pub fn avg_generation_speed_ms(&self) -> Option<u128> {
        let successful_generations: Vec<u128> = self.metrics.iter().filter_map(|entry| {
            if let MetricEvent::IpaGenerated { success: true, duration_ms, .. } = &entry.event {
                Some(*duration_ms)
            } else {
                None
            }
        }).collect();

        if successful_generations.is_empty() {
            None
        } else {
            Some(successful_generations.iter().sum::<u128>() / successful_generations.len() as u128)
        }
    }
}
