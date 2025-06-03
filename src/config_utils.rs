use std::path::PathBuf;
use directories_next::ProjectDirs;
use crate::app::IpaBuilderApp; 

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "i2sac";
const APPLICATION: &str = "IPABuilder";

// Helper to get project directories
fn get_project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
}

// Get the path to the configuration file (e.g., for app state)
pub fn get_config_dir_path() -> Option<PathBuf> { // Renamed for clarity and consistency
    get_project_dirs().map(|proj_dirs| {
        let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(config_dir) {
                log::error!("Failed to create config directory {}: {}", config_dir.display(), e);
            }
        }
        config_dir.to_path_buf() // Return the directory itself, not a specific file
    })
}

// Get the path to the data directory (e.g., for metrics)
pub fn get_data_dir_path() -> Option<PathBuf> {
    get_project_dirs().map(|proj_dirs| {
        let data_dir = proj_dirs.data_local_dir();
        if !data_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(data_dir) {
                log::error!("Failed to create data directory {}: {}", data_dir.display(), e);
            }
        }
        data_dir.to_path_buf()
    })
}

// Load application state
pub fn load_app_state(cc: &eframe::CreationContext<'_>) -> Result<IpaBuilderApp, String> {
    let config_file_path = get_config_dir_path().map(|d| d.join("app_state.json"));
    if let Some(config_path) = config_file_path {
        if config_path.exists() {
            log::info!("Loading app state from: {}", config_path.display());
            match std::fs::read_to_string(&config_path) {
                Ok(json_string) => {
                    match serde_json::from_str::<IpaBuilderApp>(&json_string) {
                        Ok(mut loaded_app) => {
                            log::info!("App state loaded successfully.");
                            loaded_app.post_load_setup(cc); 
                            Ok(loaded_app)
                        }
                        Err(e) => {
                            let msg = format!("Failed to deserialize app state from {}: {}. Using default.", config_path.display(), e);
                            log::error!("{}", msg);
                            Err(msg) 
                        }
                    }
                }
                Err(e) => {
                    let msg = format!("Failed to read app state file {}: {}. Using default.", config_path.display(), e);
                    log::error!("{}", msg);
                    Err(msg)
                }
            }
        } else {
            log::info!("No app state file found at {}. Using default.", config_path.display());
            let mut app = IpaBuilderApp::default();
            app.post_load_setup(cc);
            Ok(app) 
        }
    } else {
        let msg = "Could not determine config file path. Using default app state.".to_string();
        log::warn!("{}", msg);
        Err(msg)
    }
}
