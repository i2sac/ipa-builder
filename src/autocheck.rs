use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::app::AppConfig;

#[derive(Debug, Clone)]
pub struct AutoCheckConfig {
    pub watch_dir: PathBuf,
    pub output_dir: PathBuf,
    pub app_name: String,
    pub output_ipa_name: String,
}

#[derive(Debug, Clone)]
pub enum AutoCheckMessage {
    Status(String),
}

pub struct AutoCheckRunner {
    stop_flag: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
    rx: mpsc::Receiver<AutoCheckMessage>,
}

impl AutoCheckRunner {
    pub fn start(cfg: AutoCheckConfig) -> Result<Self, String> {
        if !cfg.watch_dir.is_dir() {
            return Err(format!("Watch directory is invalid: {}", cfg.watch_dir.display()));
        }
        if !cfg.output_dir.is_dir() {
            return Err(format!("Output directory is invalid: {}", cfg.output_dir.display()));
        }
        if cfg.app_name.trim().is_empty() {
            return Err("App name cannot be empty".to_string());
        }
        if cfg.output_ipa_name.trim().is_empty() || !cfg.output_ipa_name.to_lowercase().ends_with(".ipa") {
            return Err("Output IPA name must end with .ipa".to_string());
        }
        if cfg.output_ipa_name.contains('/') || cfg.output_ipa_name.contains('\\') {
            return Err("Output IPA name must be a file name, not a path".to_string());
        }

        let (tx, rx) = mpsc::channel::<AutoCheckMessage>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_thread = Arc::clone(&stop_flag);

        let join_handle = thread::spawn(move || {
            let _ = tx.send(AutoCheckMessage::Status(format!(
                "AutoCheck started. Watching: {}",
                cfg.watch_dir.display()
            )));

            let (event_tx, event_rx) = mpsc::channel::<notify::Result<Event>>();

            let mut watcher: RecommendedWatcher = match RecommendedWatcher::new(
                move |res| {
                    let _ = event_tx.send(res);
                },
                Config::default(),
            ) {
                Ok(w) => w,
                Err(e) => {
                    let _ = tx.send(AutoCheckMessage::Status(format!(
                        "AutoCheck watcher init error: {}",
                        e
                    )));
                    return;
                }
            };

            if let Err(e) = watcher.watch(&cfg.watch_dir, RecursiveMode::NonRecursive) {
                let _ = tx.send(AutoCheckMessage::Status(format!(
                    "AutoCheck watcher start error: {}",
                    e
                )));
                return;
            }

            while !stop_flag_thread.load(Ordering::Relaxed) {
                match event_rx.recv_timeout(Duration::from_millis(250)) {
                    Ok(Ok(ev)) => {
                        for path in ev.paths {
                            if stop_flag_thread.load(Ordering::Relaxed) {
                                break;
                            }
                            if !is_candidate_runner_zip(&path) {
                                continue;
                            }

                            let _ = tx.send(AutoCheckMessage::Status(format!(
                                "Detected candidate: {}",
                                path.display()
                            )));

                            if let Err(e) = wait_until_file_ready(&path, Duration::from_secs(15)) {
                                let _ = tx.send(AutoCheckMessage::Status(format!(
                                    "Skipped (not ready): {} ({})",
                                    path.display(),
                                    e
                                )));
                                continue;
                            }

                            let app_config = AppConfig {
                                id: "autocheck".to_string(),
                                app_name: cfg.app_name.clone(),
                                input_zip_path: path.to_string_lossy().into_owned(),
                                output_ipa_name: cfg.output_ipa_name.clone(),
                                created_at: chrono::Utc::now(),
                                last_generated_at: None,
                            };

                            match crate::ipa_logic::generate_ipa(&app_config, &cfg.output_dir) {
                                Ok(out) => {
                                    let _ = tx.send(AutoCheckMessage::Status(format!(
                                        "Generated: {}",
                                        out.display()
                                    )));
                                }
                                Err(e) => {
                                    let _ = tx.send(AutoCheckMessage::Status(format!(
                                        "Generation error for {}: {}",
                                        path.display(),
                                        e
                                    )));
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        let _ = tx.send(AutoCheckMessage::Status(format!(
                            "Watcher event error: {}",
                            e
                        )));
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }

            let _ = tx.send(AutoCheckMessage::Status("AutoCheck stopped.".to_string()));
        });

        Ok(Self {
            stop_flag,
            join_handle: Some(join_handle),
            rx,
        })
    }

    pub fn try_recv(&self) -> Option<AutoCheckMessage> {
        self.rx.try_recv().ok()
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

fn is_candidate_runner_zip(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    let file_name = match path.file_name().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return false,
    };

    let lower = file_name.to_ascii_lowercase();
    lower.starts_with("runner.app") && lower.ends_with(".zip")
}

fn wait_until_file_ready(path: &Path, max_wait: Duration) -> Result<(), String> {
    let start = std::time::Instant::now();
    let mut last_len: Option<u64> = None;

    while start.elapsed() < max_wait {
        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => {
                thread::sleep(Duration::from_millis(250));
                continue;
            }
        };

        let len = meta.len();
        if let Some(prev) = last_len {
            if prev == len {
                if std::fs::File::open(path).is_ok() {
                    return Ok(());
                }
            }
        }
        last_len = Some(len);
        thread::sleep(Duration::from_millis(400));
    }

    Err("timeout".to_string())
}
