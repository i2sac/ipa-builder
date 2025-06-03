use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::path::PathBuf;

use crate::config_utils::{get_data_dir_path};
use crate::metrics::{MetricEvent, MetricsCollector};
use egui_extras::{Column, TableBuilder};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub id: String, 
    pub app_name: String,
    pub input_zip_path: String,
    pub output_ipa_name: String,
    pub created_at: DateTime<Utc>,
    pub last_generated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct IpaBuilderApp {
    output_directory: Option<String>,
    app_configs: Vec<AppConfig>,
    status_message: String,
    dark_mode: bool,
    show_config_dialog: bool, 
    config_dialog_output_dir_input: String,

    search_query: String,
    show_add_app_dialog: bool,
    add_app_name_input: String,
    add_app_zip_path_input: Option<String>,
    add_app_output_name_input: String,

    show_edit_dialog_for_idx: Option<usize>,
    edit_app_name_input: String,
    edit_input_zip_path_input: Option<String>,
    edit_output_ipa_name_input: String,

    show_delete_confirm_for_idx: Option<usize>,

    #[serde(skip)]
    metrics_collector: MetricsCollector,
    generating_app_idx: Option<usize>,

    #[serde(skip)]
    last_generated_ipa_path: Option<PathBuf>,
}

impl IpaBuilderApp {
    pub fn post_load_setup(&mut self, _cc: &eframe::CreationContext<'_>) {
        log::info!("IpaBuilderApp::post_load_setup called.");
        self.metrics_collector = MetricsCollector::new(get_data_dir_path().expect("Failed to get data dir for metrics post-load").join("metrics.jsonl"));
    }
}

impl Default for IpaBuilderApp {
    fn default() -> Self {
        let data_dir_path = get_data_dir_path().expect("Failed to get data dir for metrics default");
        let metrics_collector = MetricsCollector::new(data_dir_path.join("metrics.jsonl"));
        
        Self {
            output_directory: None,
            app_configs: Vec::new(),
            status_message: "Welcome to IPA Builder!".to_string(),
            dark_mode: true,
            show_config_dialog: true, 
            config_dialog_output_dir_input: "".to_string(),
            metrics_collector,
            search_query: String::new(),
            show_add_app_dialog: false,
            add_app_name_input: "MyNewApp".to_string(),
            add_app_zip_path_input: None,
            add_app_output_name_input: "output.ipa".to_string(),
            show_edit_dialog_for_idx: None,
            edit_app_name_input: String::new(),
            edit_input_zip_path_input: None,
            edit_output_ipa_name_input: String::new(),
            show_delete_confirm_for_idx: None,
            generating_app_idx: None,
            last_generated_ipa_path: None,
        }
    }
}

impl eframe::App for IpaBuilderApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
            match serde_json::to_string(self) {
                Ok(json_string) => {
                    storage.set_string(eframe::APP_KEY, json_string);
                    log::trace!("App state saved via storage.set_string");
                }
                Err(e) => {
                    log::error!("Failed to serialize app state: {}", e);
                }
            }
        }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.output_directory.is_none() {
            self.show_config_dialog = true;
        }

        if self.show_config_dialog {
            self.render_config_dialog(ctx);
            return;
        }

        self.render_main_ui(ctx);
        self.render_add_app_dialog(ctx);
        self.render_edit_dialog(ctx);
        self.render_delete_confirm_dialog(ctx);
    }
}

impl IpaBuilderApp {

    fn open_folder_containing_file(&self, file_path: &PathBuf) {
        if let Some(parent_dir) = file_path.parent() {
            let command_name = if cfg!(target_os = "windows") {
                "explorer"
            } else if cfg!(target_os = "macos") {
                "open"
            } else { // Assuming Linux or other Unix-like
                "xdg-open"
            };
            match std::process::Command::new(command_name).arg(parent_dir).spawn() {
                Ok(_) => log::info!("Attempted to open folder: {}", parent_dir.display()),
                Err(e) => log::error!("Failed to open folder {}: {}", parent_dir.display(), e),
            }
        } else {
            log::warn!("File path {} has no parent directory.", file_path.display());
        }
    }

    fn record_metric(&mut self, event_type: MetricEvent) {
        self.metrics_collector.record(event_type);
    }

    fn render_main_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = false;
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.separator();
                ui.heading("IPA Builder Dashboard");
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Today's Generations: {}", self.metrics_collector.generations_today()));
                ui.separator();
                ui.label(format!("Total Generations: {}", self.metrics_collector.generations_all_time()));
                ui.separator();
                if let Some(avg_speed) = self.metrics_collector.avg_generation_speed_ms() {
                    ui.label(format!("Avg. Speed: {:.2}s", avg_speed as f64 / 1000.0));
                } else {
                    ui.label("Avg. Speed: N/A");
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("➕ Add Application").clicked() {
                    self.show_add_app_dialog = true;
                    self.add_app_name_input = format!("MyNewApp{}", self.app_configs.len() + 1);
                    self.add_app_output_name_input = format!("app{}.ipa", self.app_configs.len() + 1);
                    self.add_app_zip_path_input = None;
                }
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);
            });
            ui.separator();

            let lower_search_query = self.search_query.to_lowercase();
            let config_indices_to_display: Vec<usize> = self.app_configs.iter().enumerate()
                .filter(|(_, config)| {
                    self.search_query.is_empty() || 
                    config.app_name.to_lowercase().contains(&lower_search_query) ||
                    config.input_zip_path.to_lowercase().contains(&lower_search_query)
                })
                .map(|(idx, _)| idx)
                .collect();

            let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::auto())
                .column(Column::initial(200.0).clip(true))
                .column(Column::initial(200.0).clip(true))
                .column(Column::initial(150.0))
                .column(Column::remainder())
                .min_scrolled_height(0.0);

            table.header(20.0, |mut header| {
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Input ZIP"); });
                header.col(|ui| { ui.strong("Output IPA"); });
                header.col(|ui| { ui.strong("Created"); });
                header.col(|ui| { ui.strong("Actions"); });
            })
            .body(|mut body| {
                for &original_idx in &config_indices_to_display {
                            // Clone data needed for display to avoid borrowing `self.app_configs` in the row closure
                            let display_app_name = self.app_configs[original_idx].app_name.clone();
                            let display_last_gen_str = self.app_configs[original_idx].last_generated_at
                                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string());
                            let display_input_zip = self.app_configs[original_idx].input_zip_path.clone();
                            let display_output_ipa = self.app_configs[original_idx].output_ipa_name.clone();
                            let display_created_at = self.app_configs[original_idx].created_at.format("%Y-%m-%d %H:%M").to_string();

                            body.row(text_height + 4.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(&display_app_name);
                                    if let Some(gen_time_str) = &display_last_gen_str {
                                        ui.small(format!("Last gen: {}", gen_time_str));
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(&display_input_zip);
                                });
                                row.col(|ui| {
                                    ui.label(&display_output_ipa);
                                });
                                row.col(|ui| {
                                    ui.label(&display_created_at);
                                });
                                row.col(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("✏️").on_hover_text("Edit").clicked() {
                                            self.edit_app_name_input = self.app_configs[original_idx].app_name.clone();
                                            self.edit_input_zip_path_input = Some(self.app_configs[original_idx].input_zip_path.clone());
                                            self.edit_output_ipa_name_input = self.app_configs[original_idx].output_ipa_name.clone();
                                            self.show_edit_dialog_for_idx = Some(original_idx);
                                        }
                                        let gen_button_text = if self.generating_app_idx == Some(original_idx) {
                                            "⏳"
                                        } else {
                                            "▶️"
                                        };
                                        if ui.button(gen_button_text).on_hover_text("Generate IPA").clicked() {
                                            if self.generating_app_idx.is_none() {
                                                // Clone the AppConfig for this specific generation task
                                                let app_config_for_generation = self.app_configs[original_idx].clone();

                                                self.generating_app_idx = Some(original_idx);
                                                self.status_message = format!("Generating IPA for {}...", app_config_for_generation.app_name);
                                                let start_time = std::time::Instant::now();
                                                match crate::ipa_logic::generate_ipa(&app_config_for_generation, std::path::Path::new(self.output_directory.as_ref().unwrap())) {
                                                    Ok(output_path) => {
                                                        let duration = start_time.elapsed();
                                                        self.last_generated_ipa_path = Some(output_path.clone()); // Store the path
                                                        self.status_message = format!("IPA for '{}' generated successfully in {:.2}s at: {}", app_config_for_generation.app_name, duration.as_secs_f32(), output_path.display());
                                                        log::info!("IPA generated: {}", output_path.display());
                                                        if let Some(cfg_to_update) = self.app_configs.get_mut(original_idx) {
                                                            cfg_to_update.last_generated_at = Some(Utc::now());
                                                        }
                                                        self.record_metric(MetricEvent::IpaGenerated { 
                                                            app_name: app_config_for_generation.app_name.clone(), 
                                                            success: true, 
                                                            duration_ms: duration.as_millis(), 
                                                            output_size_bytes: std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0) 
                                                        });
                                                    }
                                                    Err(e) => {
                                                        self.status_message = format!("Error for {}: {}", app_config_for_generation.app_name, e);
                                                        log::error!("Error generating IPA for {}: {}", app_config_for_generation.app_name, e);
                                                        self.record_metric(MetricEvent::IpaGenerated { 
                                                            app_name: app_config_for_generation.app_name.clone(), 
                                                            success: false, 
                                                            duration_ms: start_time.elapsed().as_millis(), 
                                                            output_size_bytes: 0 
                                                        });
                                                    }
                                                }
                                                self.generating_app_idx = None;
                                            }
                                        }
                                        if ui.button("🗑️").clicked() {
                                            self.show_delete_confirm_for_idx = Some(original_idx);
                                        }
                                    });
                                });
                            });
                        } 
                    });
            ui.separator();
            ui.label(&self.status_message).highlight();

            if let Some(ref path) = self.last_generated_ipa_path {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("Last generated IPA:");
                    if ui.link(path.display().to_string()).on_hover_text("Click to open containing folder").clicked() {
                        self.open_folder_containing_file(path);
                    }
                });
            }
        });
    }

    fn render_add_app_dialog(&mut self, ctx: &egui::Context) {
        if self.show_add_app_dialog {
            let mut close_dialog = false;
            egui::Window::new("Add New Application")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Application Name (for display):");
                    ui.text_edit_singleline(&mut self.add_app_name_input);

                    ui.label("Runner.app.zip Path:");
                    ui.horizontal(|ui| {
                        let zip_path_display = self.add_app_zip_path_input.as_ref().map_or_else(|| "Not selected".to_string(), |p| p.clone());
                        ui.label(zip_path_display);
                        if ui.button("Browse...").clicked() {
                            match native_dialog::FileDialog::new()
                                .add_filter("Zip files", &["zip"])
                                .show_open_single_file() {
                                Ok(Some(path)) => {
                                    self.add_app_zip_path_input = Some(path.to_string_lossy().into_owned());
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    log::error!("Error opening file dialog: {:?}", e);
                                    self.status_message = format!("Error opening file dialog: {:?}. Ensure zenity or GTK utils are installed.", e);
                                }
                            }
                        }
                    });
                    
                    ui.label("Output IPA Filename (e.g., myapp_v1.ipa):");
                    ui.text_edit_singleline(&mut self.add_app_output_name_input);

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Add Application").clicked() {
                            if self.add_app_name_input.trim().is_empty() {
                                self.status_message = "Application name cannot be empty.".to_string();
                            } else if self.add_app_zip_path_input.is_none() {
                                self.status_message = "Please select an input ZIP file.".to_string();
                            } else if self.add_app_output_name_input.trim().is_empty() || !self.add_app_output_name_input.ends_with(".ipa") {
                                self.status_message = "Output filename must not be empty and end with .ipa".to_string();
                            } else {
                                let new_app = AppConfig {
                                    id: Uuid::new_v4().to_string(),
                                    app_name: self.add_app_name_input.trim().to_string(),
                                    input_zip_path: self.add_app_zip_path_input.clone().unwrap(), // Safe due to check above
                                    output_ipa_name: self.add_app_output_name_input.trim().to_string(),
                                    created_at: Utc::now(),
                                    last_generated_at: None,
                                };
                                self.app_configs.push(new_app);
                                self.status_message = format!("Application '{}' added.", self.add_app_name_input);
                                self.record_metric(MetricEvent::AppAdded { app_name: self.add_app_name_input.clone() });
                                // Reset inputs
                                self.add_app_name_input = "MyNewApp".to_string();
                                self.add_app_zip_path_input = None;
                                self.add_app_output_name_input = "output.ipa".to_string();
                                close_dialog = true;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            close_dialog = true;
                        }
                    });
                });
            if close_dialog {
                self.show_add_app_dialog = false;
            }
        }
    }

    fn render_edit_dialog(&mut self, ctx: &egui::Context) {
        if let Some(idx) = self.show_edit_dialog_for_idx {
            let mut close_dialog = false;
            let original_app_name = self.app_configs.get(idx).map_or_else(String::new, |ac| ac.app_name.clone());
            let app_id_to_edit = self.app_configs.get(idx).map(|ac| ac.id.clone());

            egui::Window::new(format!("Edit Configuration: {}", original_app_name))
                .collapsible(false)
                .resizable(true)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Application Name:");
                    ui.text_edit_singleline(&mut self.edit_app_name_input);
                    ui.add_space(5.0);

                    ui.label("Input Runner.app.zip Path:");
                    ui.horizontal(|ui| {
                        let mut display_string_for_zip_path = self.edit_input_zip_path_input.as_deref().unwrap_or("Not selected").to_string();
                        ui.add_enabled_ui(false, |dis_ui| {
                            dis_ui.text_edit_singleline(&mut display_string_for_zip_path);
                        });
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = native_dialog::FileDialog::new()
                                .add_filter("ZIP archives", &["zip"])
                                .set_filename("Runner.app.zip")
                                .show_open_single_file()
                                .unwrap_or(None)
                            {
                                self.edit_input_zip_path_input = Some(path.to_string_lossy().into_owned());
                            }
                        }
                    });
                    ui.add_space(5.0);

                    ui.label("Output IPA Filename:");
                    ui.text_edit_singleline(&mut self.edit_output_ipa_name_input);
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Save Changes").clicked() {
                            let app_name = self.edit_app_name_input.trim();
                            let zip_path = self.edit_input_zip_path_input.as_deref().map(str::trim).filter(|s| !s.is_empty());
                            let ipa_name = self.edit_output_ipa_name_input.trim();

                            if app_name.is_empty() {
                                self.status_message = "Application name cannot be empty.".to_string();
                            } else if zip_path.is_none() {
                                self.status_message = "Input ZIP path must be selected.".to_string();
                            } else if ipa_name.is_empty() || !ipa_name.ends_with(".ipa") {
                                self.status_message = "Output IPA name must not be empty and end with .ipa".to_string();
                            } else {
                                if let Some(ac) = self.app_configs.get_mut(idx) {
                                    ac.app_name = app_name.to_string();
                                    ac.input_zip_path = zip_path.unwrap().to_string(); // Safe due to check
                                    ac.output_ipa_name = ipa_name.to_string();
                                    self.status_message = format!("Configuration for '{}' updated.", ac.app_name);
                                    if let Some(id_val) = app_id_to_edit {
                                        self.record_metric(MetricEvent::AppConfigEdited { app_id: id_val });
                                    }
                                }
                                close_dialog = true;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            close_dialog = true;
                        }
                    });
                });

            if close_dialog {
                self.show_edit_dialog_for_idx = None;
                // Optionally clear edit fields or leave them for next time
                // self.edit_app_name_input = String::new();
                // self.edit_input_zip_path_input = None;
                // self.edit_output_ipa_name_input = String::new();
            }
        } else if self.show_edit_dialog_for_idx.is_some() {
             // This case handles if idx was Some but app_configs.get(idx) was None (e.g. app deleted while dialog was about to open)
             self.status_message = "Error: Could not find app to edit.".to_string();
             self.show_edit_dialog_for_idx = None; 
        }
    }

    fn render_delete_confirm_dialog(&mut self, ctx: &egui::Context) {
        if let Some(idx) = self.show_delete_confirm_for_idx {
            if let Some(app_to_delete_ref) = self.app_configs.get(idx) { 
                let app_name_for_dialog = app_to_delete_ref.app_name.clone(); // For dialog display
                let mut close_dialog = false;

                egui::Window::new(format!("Confirm Delete: '{}'", app_name_for_dialog))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(format!("Are you sure you want to delete the application '{}'?", app_name_for_dialog));
                        ui.add_space(10.0);
                        ui.label("This action cannot be undone.");
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                let deleted_app_name = self.app_configs[idx].app_name.clone(); // Capture name just before removal
                                self.app_configs.remove(idx);
                                self.status_message = format!("Application '{}' deleted.", deleted_app_name);
                                self.metrics_collector.record(MetricEvent::AppRemoved { app_name: deleted_app_name });
                                close_dialog = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close_dialog = true;
                            }
                        });
                    });

                if close_dialog {
                    self.show_delete_confirm_for_idx = None;
                }
            } else {
                self.show_delete_confirm_for_idx = None; // Index out of bounds, close dialog
                self.status_message = "Error: Could not find app to delete.".to_string();
            }
        }
    }

    fn render_config_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Initial Configuration - Output Directory")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Please select a default output directory for your generated IPA files.");
                ui.horizontal(|ui| {
                    ui.label("Output Directory:");
                    ui.text_edit_singleline(&mut self.config_dialog_output_dir_input);
                    if ui.button("Browse...").clicked() {
                        match native_dialog::FileDialog::new().show_open_single_dir() {
                            Ok(Some(path)) => {
                                self.config_dialog_output_dir_input = path.to_string_lossy().to_string();
                                self.status_message = "Directory selected.".to_string();
                            }
                            Ok(None) => {
                                log::info!("Directory selection cancelled by user.");
                                self.status_message = "Directory selection cancelled.".to_string();
                            }
                            Err(e) => {
                                log::error!("Error opening directory dialog: {:?}", e);
                                self.status_message = format!("Error opening directory dialog: {:?}. Ensure zenity or GTK utils are installed.", e);
                            }
                        }
                    }
                });
                
                if ui.button("Save Configuration").clicked() {
                    let path = PathBuf::from(&self.config_dialog_output_dir_input);
                    if path.is_dir() {
                        self.output_directory = Some(path.to_string_lossy().into_owned());
                        self.show_config_dialog = false;
                        self.status_message = "Output directory configured.".to_string();
                        // self.save_state(); // Removed, eframe::App::save handles state persistence
                        self.record_metric(MetricEvent::OutputDirectorySet);
                    } else {
                        self.status_message = "Invalid directory selected. Please choose a valid directory.".to_string();
                    }
                }
                ui.label(&self.status_message);
            });
    }
}

