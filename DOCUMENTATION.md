# 📚 IPA Builder - Developer Documentation 🤓

Welcome to the developer documentation for the **IPA Builder by i2sac** application! This guide is intended to walk you through the project's structure, core logic, and how various features were implemented. The goal is to provide enough insight for another developer to understand, maintain, or even rebuild a similar application from scratch. Let's dive in! 🏊‍♂️

## 📝 Table of Contents

1.  [🌟 Introduction & Goals](#-introduction--goals)
2.  [🛠️ Prerequisites & Setup](#️-prerequisites--setup)
3.  [🏗️ Project Structure Overview](#️-project-structure-overview)
4.  [⚙️ Core IPA Generation Logic (`src/ipa_logic.rs`)](#️-core-ipa-generation-logic-srcipa_logicrs)
    *   [Understanding the IPA Format](#understanding-the-ipa-format)
    *   [Step-by-step Conversion Process](#step-by-step-conversion-process)
    *   [Key Functions and Error Handling](#key-functions-and-error-handling)
5.  [🖼️ Application State & GUI (`src/app.rs`)](#️-application-state--gui-srcapprs)
    *   [`IpaBuilderApp` Struct: The Heart of the App](#ipabuilderapp-struct-the-heart-of-the-app)
    *   [State Management (App Configurations, UI State)](#state-management-app-configurations-ui-state)
    *   [Implementing `eframe::App`](#implementing-eframeapp)
    *   [Rendering the UI with `egui`](#rendering-the-ui-with-egui)
        *   [Main View (App List, Buttons)](#main-view-app-list-buttons)
        *   [Settings View](#settings-view)
        *   [Edit/Add Dialogs](#editadd-dialogs)
        *   [Theme Switching (Light/Dark)](#theme-switching-lightdark)
6.  [🚀 Main Application Entry Point (`src/main.rs`)](#-main-application-entry-point-srcmainrs)
    *   [Setting up `eframe`](#setting-up-eframe)
    *   [Loading the Application Icon](#loading-the-application-icon)
    *   [Initializing and Running the App](#initializing-and-running-the-app)
7.  [💾 Configuration & Data Persistence](#-configuration--data-persistence)
    *   [Storing App Configurations](#storing-app-configurations)
    *   [Saving and Loading State with `serde`](#saving-and-loading-state-with-serde)
    *   [Application Directory (`directories-next`)](#application-directory-directories-next)
8.  [📊 Metrics Collection (`src/metrics.rs`)](#-metrics-collection-srcmetricsrs)
    *   [`MetricEvent` Enum](#metricevent-enum)
    *   [`MetricsCollector` Struct](#metricscollector-struct)
    *   [Storing Metrics Locally](#storing-metrics-locally)
9.  [🎨 Icon Handling](#-icon-handling)
    *   [Loading PNG Icon](#loading-png-icon)
    *   [Setting Window Icon with `eframe`](#setting-window-icon-with-eframe)
10. [❗ Error Handling](#-error-handling)
    *   [Custom Error Types (`AppError` in `ipa_logic.rs`)](#custom-error-types-apperror-in-ipa_logicrs)
    *   [Displaying Errors in the UI](#displaying-errors-in-the-ui)
11. [📦 Building and Running](#-building-and-running)
    *   [Development vs. Release Builds](#development-vs-release-builds)
12. [🔮 Future Enhancements & Roadmap](#-future-enhancements--roadmap)

---

## 1. 🌟 Introduction & Goals

The primary goal of **IPA Builder** is to provide a simple, cross-platform desktop utility to automate the conversion of `Runner.app.zip` files (common in Flutter iOS builds from CI/CD like Codemagic) into installable `.ipa` files. This saves developers from the manual, error-prone process of unzipping, renaming, re-zipping, and renaming again.

**Key Objectives During Development:**

*   **Ease of Use:** A straightforward GUI that requires minimal user input.
*   **Efficiency:** Quick conversion of app bundles.
*   **Persistence:** Remembering app configurations for repeated use.
*   **Cross-Platform:** Aiming for compatibility with Windows, macOS, and Linux.
*   **Modularity:** Separating business logic (IPA creation) from UI logic.

## 2. 🛠️ Prerequisites & Setup

To build and run this project from scratch, or to contribute, you'll need:

*   **Rust:** The latest stable version is recommended. Install it via [rustup.rs](https://rustup.rs/).
*   **Git:** For cloning the repository.
*   **Basic understanding of Rust programming concepts.**
*   **(Optional) An IDE with Rust support:** VS Code with the `rust-analyzer` extension is a popular choice.

**Initial Project Setup (if starting from scratch):**

```bash
# Create a new Rust binary project
cargo new ipa_builder --bin
cd ipa_builder

# Add initial dependencies to Cargo.toml
# (eframe, egui, serde, serde_json, zip, uuid, chrono, etc.)
# For example:
# cargo add eframe egui serde serde_json zip uuid chrono native-dialog directories-next tempfile log env_logger thiserror walkdir egui_extras image
```

## 3. 🏗️ Project Structure Overview

The project follows a standard Rust binary layout:

```
ipa-builder/
├── Cargo.toml      # Manages project dependencies and metadata
├── Cargo.lock      # Generated lockfile for reproducible builds
├── assets/         # Static assets like the application icon
│   └── icon.png
├── src/
│   ├── main.rs     # Main application entry point, sets up eframe
│   ├── app.rs      # Defines the IpaBuilderApp struct, GUI logic, and state management
│   ├── ipa_logic.rs # Core logic for IPA file generation
│   ├── metrics.rs  # Logic for collecting and storing usage metrics
│   └── errors.rs   # (This was integrated into ipa_logic.rs as AppError enum)
├── target/         # Build artifacts (generated by cargo)
├── README.md       # Project overview and user guide
├── DOCUMENTATION.md # This file!
└── LICENSE         # Project license information
```

*   **`main.rs`**: Initializes the `eframe` environment and runs the `IpaBuilderApp`.
*   **`app.rs`**: Contains the main application struct (`IpaBuilderApp`) which implements `eframe::App`. This file handles all GUI rendering (using `egui`), state management (like the list of app configurations, UI settings), and interactions between the UI and the backend logic.
*   **`ipa_logic.rs`**: Houses the core functionality for converting a `Runner.app.zip` into an `.ipa` file. This module is designed to be independent of the UI. It also includes the custom `AppError` enum for error handling within this module.
*   **`metrics.rs`**: Manages the collection and storage of anonymous usage data.
*   **`assets/`**: Contains static files, primarily the application icon.

---

## 4. ⚙️ Core IPA Generation Logic (`src/ipa_logic.rs`)

This module is the heart of the application's functionality: converting a `Runner.app.zip` file into a usable `.ipa` file. It's designed to be self-contained and independent of the UI layer.

### Understanding the IPA Format

An `.ipa` file is essentially a ZIP archive containing the application bundle. The key requirement is that the application bundle (e.g., `YourApp.app`) must reside inside a directory named `Payload` at the root of the archive.

```
MyAwesomeApp.ipa (which is a .zip file)
└── Payload/
    └── YourApp.app/
        ├── Info.plist
        ├── YourAppExecutable
        ├── Frameworks/
        └── ... (other app bundle contents)
```

### Step-by-step Conversion Process

The `generate_ipa` function in `src/ipa_logic.rs` orchestrates the conversion. Here's a breakdown of the steps:

1.  **Temporary Directory Creation 📁:**
    *   A temporary directory is created using the `tempfile` crate. This ensures that all intermediate files are isolated and cleaned up automatically, even if errors occur.

2.  **Input ZIP Extraction 📤:**
    *   The provided `Runner.app.zip` (or user-selected input zip) is extracted into a subdirectory within the temporary directory (e.g., `temp_dir/extracted_zip/`).
    *   The `zip` crate is used for this. The extraction logic iterates through each file in the archive and writes it to the filesystem.

3.  **Locating the `.app` Bundle 🔎:**
    *   After extraction, the code needs to find the actual `.app` bundle. `Runner.app.zip` files from Codemagic often have a structure like `Runner.app/Runner.app` or similar. The logic looks for a directory ending with `.app` within the extracted contents. 
    *   It prioritizes finding a nested `.app` directory if the top-level extracted folder is also an `.app` directory (e.g. `extracted_zip/Runner.app/Runner.app`). If not, it looks for any `.app` directory at the first level of the extracted content.
    *   This step is crucial because the *parent* of this located `.app` bundle (or the `.app` bundle itself if it's at the root of the extraction) needs to be renamed to `Payload`.

4.  **Creating the `Payload` Structure 🏗️:**
    *   A new directory named `Payload` is created directly inside the main temporary directory (e.g., `temp_dir/Payload`).
    *   The located `.app` bundle (from step 3) is then **moved** into this `Payload` directory. So, the structure becomes `temp_dir/Payload/YourApp.app`.

5.  **Zipping the `Payload` Directory  compressing_input:**
    *   The `Payload` directory (now containing the `.app` bundle) is compressed into a new ZIP file. This ZIP file is initially created with a temporary name (e.g., `temp_ipa.zip`) within the specified final output directory.
    *   The `zip` crate's `ZipWriter` is used, and functions are included to recursively add files and directories to the archive, maintaining their relative paths within `Payload`.

6.  **Renaming to `.ipa` 🏷️:**
    *   The newly created temporary ZIP file (e.g., `temp_ipa.zip`) is renamed to the user-specified output IPA filename (e.g., `MyAwesomeApp.ipa`). This final file is located in the user's chosen output directory.

7.  **Cleanup 🧹:**
    *   The `tempfile::TempDir` automatically removes the temporary directory and all its contents when it goes out of scope, ensuring no intermediate files are left behind.

### Key Functions and Error Handling

*   **`generate_ipa(app_config: &AppConfig, output_directory: &Path) -> Result<PathBuf, AppError>`:**
    *   The main public function of the module.
    *   Takes an `AppConfig` (containing input zip path and output IPA name) and the target `output_directory`.
    *   Returns a `Result` with the `PathBuf` to the successfully generated IPA file or an `AppError`.

*   **`zip_dir(it: &mut dyn Iterator<Item = DirEntry>, prefix: &str, writer: &mut ZipWriter<File>, method: zip::CompressionMethod) -> zip::result::ZipResult<()>`:**
    *   A helper function (often made private or part of an internal module) to recursively add files from a directory to a ZIP archive. It's used to create the final IPA from the `Payload` directory.

*   **`AppError` Enum:**
    *   A custom error enum defined using `thiserror` to provide specific error types for different failure points in the IPA generation process (e.g., `IoError`, `ZipError`, `AppBundleNotFound`, `TempDirError`).
    *   This allows the UI layer to display more informative error messages to the user.

This structured approach ensures that the IPA generation is robust and handles potential issues gracefully.

---

## 5. 🖼️ Application State & GUI (`src/app.rs`)

This file is the most substantial part of the UI application, defining the main application struct `IpaBuilderApp` and handling all aspects of the user interface and application state.

### `IpaBuilderApp` Struct: The Heart of the App

The `IpaBuilderApp` struct holds all the data the application needs to operate and persist across sessions. Key fields include:

*   **`app_configs: Vec<AppConfig>`**: A list of user-defined application configurations. Each `AppConfig` typically stores:
    *   `id: Uuid`: A unique identifier for the configuration.
    *   `app_name: String`: A user-friendly name for this app configuration.
    *   `input_zip_path: String`: The path to the input `Runner.app.zip`.
    *   `output_ipa_name: String`: The desired filename for the output `.ipa` (e.g., `MyApp-v1.0.ipa`).
    *   `created_at: DateTime<Utc>`: Timestamp of creation.
    *   `last_generated_at: Option<DateTime<Utc>>`: Timestamp of the last successful generation.
*   **`output_directory: Option<String>`**: The default directory where generated IPA files will be saved. This is configured by the user.
*   **`status_message: String`**: Displays feedback to the user (e.g., success/error messages).
*   **`last_generated_ipa_path: Option<PathBuf>`**: Stores the path of the most recently generated IPA, used for the "Open Folder" feature. (Not serialized).
*   **UI State Fields**: Various fields to manage the state of the UI, such as:
    *   `dark_mode: bool`: Tracks whether dark or light theme is active.
    *   `show_add_app_dialog: bool`: Controls visibility of the "Add App" dialog.
    *   `add_app_name_input: String`, `add_input_zip_path_input: Option<String>`, `add_output_ipa_name_input: String`: Input buffers for the "Add App" dialog.
    *   `show_edit_dialog_for_idx: Option<usize>`: Tracks which app config is being edited.
    *   `edit_app_name_input: String`, `edit_input_zip_path_input: Option<String>`, `edit_output_ipa_name_input: String`: Input buffers for the "Edit App" dialog.
    *   `show_settings_dialog: bool`: Controls visibility of the settings panel/dialog.
    *   `show_delete_confirmation_for_idx: Option<usize>`: Tracks which app config is pending deletion confirmation.
    *   `search_query: String`: Stores the current text in the search bar.
*   **`metrics_collector: MetricsCollector`**: An instance for recording usage metrics.

The struct derives `serde::Serialize` and `serde::Deserialize` to allow easy saving and loading of its state (except for fields explicitly skipped like `last_generated_ipa_path` or `metrics_collector` which has its own persistence).

### State Management

*   **App Configurations (`app_configs`)**: Managed as a `Vec<AppConfig>`. Users can add, edit, and delete these configurations through the UI. Each action updates this vector.
*   **UI Dialogs**: Boolean flags (e.g., `show_add_app_dialog`) control the visibility of modal dialogs. Input fields for these dialogs are stored as separate string buffers in `IpaBuilderApp`.
*   **File Paths**: Paths are generally stored as `String` for serialization and ease of use with `egui` input fields, then converted to `Path` or `PathBuf` when interacting with the filesystem or `ipa_logic` module.
*   **Persistence**: The entire `IpaBuilderApp` state (or most of it) is serialized to JSON and saved when the application closes (via the `save` method of `eframe::App`). It's loaded when the app starts.

### Implementing `eframe::App`

`IpaBuilderApp` implements the `eframe::App` trait, which is the core of an `egui`-based application:

*   **`fn new(cc: &eframe::CreationContext) -> Self`**: The constructor. 
    *   It attempts to load previously saved application state from storage using `cc.storage`. 
    *   If no saved state is found, it initializes with default values (e.g., an empty list of app configs, dark mode potentially based on system preference).
    *   Sets up `egui` visual style (fonts, initial theme).
    *   Initializes the `MetricsCollector`.
*   **`fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)`**: Called each frame to update the UI and handle logic.
    *   Sets the visual style (light/dark mode) based on `self.dark_mode`.
    *   Defines the layout of the application (e.g., top panel for global actions, central panel for the main content).
    *   Renders all UI elements: buttons, labels, text inputs, tables, dialogs.
    *   Handles user input: button clicks, text entry, file dialog interactions.
    *   Calls `ipa_logic::generate_ipa` when a "Generate" button is clicked.
    *   Updates `self.status_message` based on actions.
*   **`fn save(&mut self, storage: &mut dyn eframe::Storage)`**: Called when the application is about to close.
    *   Serializes `self` (the `IpaBuilderApp` instance) to a JSON string.
    *   Saves this JSON string to persistent storage using `storage.set_string()`.
    *   Also triggers saving any pending metrics via `self.metrics_collector.save_metrics_to_file()`.
*   **`fn name(&self) -> &str`**: Returns the application name, used for the window title.

### Rendering the UI with `egui`

The `update` method is where all UI rendering happens. `egui`'s immediate mode paradigm is used.

*   **Main Layout (`render_main_ui` method, called from `update`):**
    *   **Top Panel (`egui::TopBottomPanel::top`):** Contains global elements like a "Settings" button, "Add App Configuration" button, and potentially a search bar.
    *   **Central Panel (`egui::CentralPanel::default()`):** 
        *   Displays the list of `AppConfig` items, often using `egui_extras::Table` or a scrollable area with horizontally laid out items for each app.
        *   Each app entry shows its name, input/output paths, and action buttons ("Generate", "Edit", "Delete").
        *   A status message area at the bottom displays feedback.
        *   A clickable link to the last generated IPA path appears after successful generation.
*   **Dialogs (`render_..._dialog` methods):**
    *   Modal dialogs (e.g., `egui::Window::new(...).modal(true).anchor(...)`) are rendered conditionally based on boolean flags (e.g., `self.show_add_app_dialog`).
    *   **Add/Edit App Dialog:** Contains `TextEdit` widgets for app name, output IPA name, and a button to browse for the input ZIP file (using `native_dialog::FileDialog`). Includes "Save" and "Cancel" buttons.
    *   **Settings Dialog/Panel:** Allows changing the output directory and toggling the theme.
    *   **Delete Confirmation Dialog:** A simple dialog with "Yes" and "No" buttons to confirm deletion of an app configuration.
*   **Theme Switching:**
    *   A boolean `self.dark_mode` controls the theme.
    *   In `update`, `ctx.set_visuals()` is called with either `egui::Visuals::dark()` or `egui::Visuals::light()`.
    *   A button/switch in the settings UI toggles `self.dark_mode`.
*   **Search Functionality:**
    *   A `TextEdit` widget for the search query (`self.search_query`).
    *   The list of `app_configs` displayed is filtered based on whether `app_name`, `input_zip_path`, or `output_ipa_name` contains the search query (case-insensitive).
*   **File Dialogs:**
    *   The `native_dialog` crate is used to open system native file dialogs for selecting the input `Runner.app.zip` and the output directory.

This separation of concerns within `app.rs` (state, UI rendering logic, dialog management) helps keep the codebase organized, even as features are added.

---

## 6. 🚀 Main Application Entry Point (`src/main.rs`)

This file is the starting point of the application. Its primary responsibilities are to configure and launch the `eframe` application window, load the application icon, and run the `IpaBuilderApp`.

### Setting up `eframe`

*   **`main()` function:** The entry point of the Rust program.
*   **Logger Initialization:** `env_logger::init()` is typically called early to enable logging throughout the application. This helps in debugging and monitoring application behavior.
*   **`eframe::NativeOptions`:** An instance of `NativeOptions` is created to configure the native window.
    *   **Window Title:** The `viewport` field's `inner_builder` is used to set the window title (e.g., "IPA Builder by i2sac").
    *   **Icon:** The `icon` field is set using the `load_icon` function (see below). This provides the application with a custom window icon.
    *   **Initial Window Size:** Can be configured here if needed (e.g., `initial_window_size`).
    *   **Persistence:** `eframe` handles saving window position and size by default if not disabled.

### Loading the Application Icon (`load_icon` function)

*   A helper function, `load_icon()`, is responsible for loading the application icon from an embedded PNG file.
*   **Embedding the Icon:** The icon (e.g., `assets/icon.png`) is included in the binary at compile time using `include_bytes!("../../assets/icon.png")`.
*   **Decoding the Image:** The `image` crate is used to decode the PNG bytes: `image::load_from_memory(ICON_BYTES)`.
*   **Converting to `IconData`:** The decoded image (which is an `image::DynamicImage`) is converted to RGBA8 format (`to_rgba8()`). The raw pixel data, width, and height are then used to construct an `eframe::IconData` instance.
*   **Error Handling:** If icon loading fails, it logs an error and returns `None`, allowing the application to start without an icon.

### Initializing and Running the App

*   **`eframe::run_native`:** This is the main function from `eframe` that starts the application.
    *   **`app_name`:** A string for the application's name, often used for default save locations.
    *   **`options`:** The `NativeOptions` configured earlier.
    *   **`Box::new(|cc| Box::new(IpaBuilderApp::new(cc)))`:** This is a closure that creates an instance of our `IpaBuilderApp`.
        *   `cc` is the `eframe::CreationContext`, which provides access to storage (`cc.storage`), `egui::Context` (`cc.egui_ctx`), and the integration frame (`cc.integration_frame`).
        *   `IpaBuilderApp::new(cc)` calls the constructor of our main application struct, passing the creation context so it can load its state, set up visuals, etc.

Once `eframe::run_native` is called, `eframe` takes over the main loop, calling the `update` method of `IpaBuilderApp` on each frame.

**Example structure of `main.rs`:**

```rust
// src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console window on Windows in release

mod app; // Assuming app.rs is in the same directory or src/
mod ipa_logic; // If directly used or for types
mod metrics;   // If directly used or for types

use app::IpaBuilderApp;
use eframe::icon_data::IconData;
use image::GenericImageView;

const ICON_BYTES: &[u8] = include_bytes!("../../assets/icon.png");

fn load_icon() -> Option<IconData> {
    match image::load_from_memory(ICON_BYTES) {
        Ok(image) => {
            let (width, height) = image.dimensions();
            let rgba = image.to_rgba8().into_raw();
            Some(IconData {
                rgba,
                width,
                height,
            })
        }
        Err(e) => {
            log::error!("Failed to load application icon: {}", e);
            None
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Initialize logger
    log::info!("Starting IPA Builder application");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 700.0]) // Initial size
            .with_min_inner_size([400.0, 300.0])
            .with_icon(load_icon().unwrap_or_default()), // Set icon
        ..
        Default::default()
    };

    eframe::run_native(
        "IPA Builder by i2sac", // App name
        options,
        Box::new(|cc| {
            // Customize egui visuals (fonts, etc.) here if needed
            // For example, cc.egui_ctx.set_fonts(...);
            Box::new(IpaBuilderApp::new(cc))
        }),
    )
}
```
This setup provides a robust starting point for an `egui` application, handling window configuration, icon loading, and the main application loop.

---

## 7. 💾 Configuration & Data Persistence

For a good user experience, it's crucial that the application remembers user settings and created configurations across sessions. This is achieved through serialization and by leveraging `eframe`'s built-in storage capabilities.

### Storing App Configurations

*   **`AppConfig` Struct:** As detailed in Section 5, the `AppConfig` struct (defined in `app.rs` or its own module if it grows complex) holds all information for a single IPA generation setup (ID, name, input path, output name, timestamps).
    *   This struct derives `serde::Serialize` and `serde::Deserialize`.
*   **`IpaBuilderApp::app_configs: Vec<AppConfig>`:** The main application struct holds a vector of these `AppConfig` instances.

### Saving and Loading State with `serde` and `eframe`

*   **Serialization (`serde`):** The entire `IpaBuilderApp` struct derives `serde::Serialize` and `serde::Deserialize`. This allows the whole application state (including the `Vec<AppConfig>`, UI settings like `dark_mode`, the `output_directory`, etc.) to be converted to a format like JSON and back.
    *   Fields that shouldn't be persisted or cannot be easily serialized (like `metrics_collector` which has its own persistence, or `last_generated_ipa_path` which is runtime data) can be skipped using `#[serde(skip)]` or `#[serde(skip_serializing, skip_deserializing)]`.
*   **`eframe::App::save()` method:**
    *   This method is called by `eframe` automatically when the application is about to close.
    *   Inside this method, `self` (the `IpaBuilderApp` instance) is serialized to a JSON string using `serde_json::to_string(self)`.
    *   The resulting JSON string is then saved using `storage.set_string(eframe::APP_KEY, json_string)`. `eframe::APP_KEY` is a default key, but a custom one could be used.
    *   `eframe` handles the actual writing to a platform-specific persistent location (e.g., application support directories).
*   **`eframe::App::new()` method (Loading):**
    *   In the `IpaBuilderApp::new(cc: &eframe::CreationContext)` constructor, the application attempts to load its previous state.
    *   `cc.storage.and_then(|s| s.get_string(eframe::APP_KEY))` retrieves the JSON string saved during the last session.
    *   If a string is found, `serde_json::from_str(&json_string)` is used to deserialize it back into an `IpaBuilderApp` instance.
    *   If no saved state is found (e.g., first launch) or deserialization fails, the application initializes with `IpaBuilderApp::default()` or some other default state.

### Application Directory (`directories-next`)

While `eframe` handles the storage of its own state (window size/position and the string set by `storage.set_string`), other files like the metrics log (`metrics.jsonl`) or potentially a separate configuration file (if not using `eframe`'s storage for everything) need a dedicated location.

*   The `directories-next` crate was initially considered and used to find platform-specific data directories.
*   **`AppDirs::new(Some("ipa_builder"), Some("i2sac"), true)`** (or similar, depending on the crate used, like `directories::ProjectDirs::from("com", "i2sac", "IPABuilder")`) can be used to get paths for:
    *   **User Data Directory:** Ideal for storing user-specific data like `metrics.jsonl` or other persistent state not managed by `eframe`'s simple key-value store.
    *   **Configuration Directory:** Could also be used, though `eframe`'s storage is often sufficient for app settings.
*   The path obtained (e.g., `app_dirs.data_dir()`) is then used to construct the full path to files like `metrics.jsonl`.
*   This ensures that files are stored in standard locations appropriate for each operating system (e.g., `~/.local/share` on Linux, `~/Library/Application Support` on macOS, `%APPDATA%` on Windows).

**Example of `save` and `new` (simplified):**

```rust
// In IpaBuilderApp impl

// Constructor (part of eframe::App::new)
fn new(cc: &eframe::CreationContext) -> Self {
    if let Some(storage) = cc.storage {
        if let Some(json_state) = storage.get_string(eframe::APP_KEY) {
            match serde_json::from_str(&json_state) {
                Ok(state) => {
                    log::info!("Loaded saved application state.");
                    return state; // Return the loaded state
                }
                Err(e) => {
                    log::error!("Failed to deserialize saved state: {}. Using default.", e);
                }
            }
        }
    }
    log::info!("No saved state found or error loading. Using default state.");
    Self::default() // Or some other default initialization
}

// Save method (eframe::App::save)
fn save(&mut self, storage: &mut dyn eframe::Storage) {
    match serde_json::to_string_pretty(self) { // Using pretty for readability if desired
        Ok(json_string) => {
            storage.set_string(eframe::APP_KEY, json_string);
            log::info!("Application state saved.");
        }
        Err(e) => {
            log::error!("Failed to serialize app state for saving: {}", e);
        }
    }
    // Also save metrics explicitly if they are not part of the main struct's serialization
    // self.metrics_collector.save_metrics_to_file(); 
}
```

This persistence mechanism ensures that user configurations and settings are not lost between application runs, providing a seamless experience.

---

## 8. 📊 Metrics Collection (`src/metrics.rs`)

To understand application usage and identify areas for improvement, a simple local metrics collection system was implemented. The goal is to gather anonymous data about how the application is used, which could later be optionally sent to a server if the user consents (though server-side upload is not implemented in the current version).

### `MetricEvent` Enum

This enum defines the different types of events that are tracked:

*   **`AppLaunched`**: Recorded when the application starts.
*   **`AppClosed`**: Recorded when the application is about to close (e.g., in the `save` method).
*   **`ThemeChanged { dark_mode: bool }`**: When the user switches between light and dark themes.
*   **`OutputDirectorySet`**: When the user sets or changes the default output directory.
*   **`AppConfigAdded { app_id: Uuid }`**: When a new app configuration is successfully added.
*   **`AppConfigEdited { app_id: Uuid }`**: When an existing app configuration is modified.
*   **`AppConfigDeleted { app_id: Uuid }`**: When an app configuration is deleted.
*   **`IpaGenerated { app_name: String, success: bool, duration_ms: u128, output_size_bytes: u64 }`**: When an IPA generation attempt is made, recording its success, duration, and output file size.

Each variant can carry relevant data for that specific event. The enum derives `serde::Serialize` and `serde::Deserialize` for easy storage.

### `MetricEntry` Struct

Each recorded event is wrapped in a `MetricEntry` struct:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetricEntry {
    pub timestamp: DateTime<Utc>,
    pub event_type: MetricEvent,
    // Potentially other common fields like session_id, user_id (if implemented)
}
```

*   **`timestamp`**: Records when the event occurred (`chrono::DateTime<Utc>`).
*   **`event_type`**: An instance of the `MetricEvent` enum.

### `MetricsCollector` Struct

This struct is responsible for managing the collection and storage of metrics:

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct MetricsCollector {
    #[serde(skip)] // Entries are not directly serialized with the collector itself
    entries: Vec<MetricEntry>,
    #[serde(skip)] // Path is runtime data
    metrics_file_path: Option<PathBuf>,
}
```

*   **`entries: Vec<MetricEntry>`**: A vector holding `MetricEntry` instances collected during the current session. This field is skipped during serialization of `MetricsCollector` itself because entries are written to a file incrementally or on save.
*   **`metrics_file_path: Option<PathBuf>`**: The path to the local file where metrics are stored (e.g., `metrics.jsonl`). This is also skipped during serialization and set at runtime.

**Key Methods:**

*   **`new(storage_dir: &Path) -> Self`**:
    *   Constructor that initializes the `MetricsCollector`.
    *   Constructs the `metrics_file_path` (e.g., `storage_dir.join("metrics.jsonl")`).
    *   It does *not* load previous metrics from the file into `self.entries` in the current design; it primarily focuses on appending new metrics.
*   **`record(&mut self, event_type: MetricEvent)`**:
    *   Creates a new `MetricEntry` with the current timestamp and the given `event_type`.
    *   Adds this entry to the `self.entries` vector.
    *   Logs the metric event.
*   **`save_metrics_to_file(&mut self)`**:
    *   This method is called, for example, when the application is closing (from `IpaBuilderApp::save`).
    *   It appends all entries currently in `self.entries` to the `metrics_file_path`.
    *   The metrics are stored in a JSON Lines format (`.jsonl`), where each line is a separate JSON object representing a `MetricEntry`. This format is robust and easy to append to.
    *   After successfully writing, `self.entries` is cleared to avoid duplicate writes in the same session if `save` were called multiple times.
    *   Handles file I/O errors.

### Storing Metrics Locally

*   **File Format:** JSON Lines (`.jsonl`). Each line in the file is a complete JSON representation of a `MetricEntry`.
    *   Example line in `metrics.jsonl`:
        ```json
        {"timestamp":"2023-10-27T10:30:00Z","event_type":{"AppLaunched":null}}
        ```
*   **Location:** The metrics file is stored in the application's user data directory, obtained using a crate like `directories-next` (e.g., `~/.local/share/ipa_builder/metrics.jsonl` on Linux).
*   **Appending:** New metrics are appended to the file, making it suitable for ongoing collection without needing to read and parse the entire file each time.

This local metrics system provides valuable insights into application usage patterns while keeping data on the user's machine. Future enhancements could include options for users to view or export their metrics, or to opt-in to sending them to a developer-managed server for aggregated analysis.

---

## 9. 🎨 Icon Handling

A custom application icon enhances the user experience and brand identity. Here's how it's managed in IPA Builder:

### Storing the Icon

*   The application icon is a PNG file (e.g., `icon.png`).
*   It's stored in an `assets/` directory at the root of the project (i.e., `assets/icon.png`).

### Loading PNG Icon at Compile Time

*   To ensure the icon is always available and doesn't need to be distributed as a separate file, it's embedded directly into the application binary at compile time.
*   This is achieved using the `include_bytes!` macro in `src/main.rs`:
    ```rust
    const ICON_BYTES: &[u8] = include_bytes!("../../assets/icon.png");
    ```
    This line reads the raw bytes of the icon file and makes them available as a static byte slice (`&[u8]`) within the program.

### Decoding and Converting the Icon

*   A helper function, typically named `load_icon()` in `src/main.rs`, handles the conversion of these raw bytes into a format `eframe` can use.
*   **`image` Crate:** The `image` crate is used for decoding.
    *   `image::load_from_memory(ICON_BYTES)` attempts to parse the byte slice, inferring the format (PNG in this case).
*   **Conversion to `IconData`:**
    *   If decoding is successful, an `image::DynamicImage` is returned.
    *   This is then converted to an RGBA8 pixel format using `image.to_rgba8()`.
    *   The raw pixel data (`Vec<u8>`), along with the image's `width` and `height` (obtained via `image.dimensions()`), are used to construct an `eframe::icon_data::IconData` struct.
    ```rust
    // Simplified from load_icon() in main.rs
    let image = image::load_from_memory(ICON_BYTES).unwrap();
    let (width, height) = image.dimensions();
    let rgba_data = image.to_rgba8().into_raw(); // Get Vec<u8>
    let icon_data = eframe::icon_data::IconData {
        rgba: rgba_data,
        width,
        height,
    };
    ```

### Setting Window Icon with `eframe`

*   The `IconData` instance created by `load_icon()` is then passed to `eframe::NativeOptions` when the application is initialized in `src/main.rs`:
    ```rust
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(load_icon().unwrap_or_default()), // Set the icon here
        // ... other options
    };
    ```
*   `eframe` uses this `IconData` to set the window icon for the application on supported platforms.
*   If `load_icon()` fails (e.g., icon file is corrupted or not found during compilation if `include_bytes!` path was wrong), it typically returns `None` or a default `IconData`, and `eframe` will use a default system icon.

This process ensures the application has a custom icon embedded within it, simplifying distribution and presentation.

---

## 10. ❗ Error Handling

Robust error handling is essential for a good user experience, ensuring the application behaves gracefully when things go wrong. IPA Builder employs several strategies for managing errors.

### Core IPA Generation Errors (`ipa_logic.rs`)

*   The `ipa_logic::generate_ipa` function returns a `Result<PathBuf, anyhow::Error>` (or a custom error type that implements `std::error::Error`).
*   **`anyhow::Error`**: This crate is commonly used for application-level error handling in Rust. It allows for easy wrapping of different error types and provides context.
*   **Specific Errors**: Inside `generate_ipa`, errors can arise from:
    *   File I/O (reading ZIP, creating temp dirs, writing files, creating final IPA).
    *   ZIP manipulation (invalid archive, missing expected files like `Runner.app`).
    *   Directory operations (renaming, creating `Payload` folder).
*   These specific errors are typically converted into `anyhow::Error` using `anyhow::bail!` or `anyhow::Context` to add descriptive messages.

### UI Error Display (`app.rs`)

*   When `generate_ipa` is called from `IpaBuilderApp` (e.g., when the "Generate IPA" button is clicked):
    *   The `Result` is matched.
    *   **On `Ok(output_path)`**: A success message is stored in `self.status_message` (e.g., `format!("✅ Successfully generated: {}", output_path.display())`). The `last_generated_ipa_path` is also updated.
    *   **On `Err(e)`**: An error message is stored in `self.status_message` (e.g., `format!("❌ Error generating IPA: {}", e)`). The error is also logged using `log::error!`.
*   The `self.status_message` (an `Option<String>`) is then displayed prominently in the UI, providing immediate feedback to the user.

### File Dialog and Path Errors

*   **`native_dialog`**: When using file dialogs (e.g., for selecting `Runner.app.zip` or the output directory):
    *   These dialogs can be cancelled by the user, which is not an error but results in `None`.
    *   If a path is selected, it's validated (e.g., checking if it's a directory or a `.zip` file as appropriate).
    *   Errors during these operations (e.g., permission issues if trying to access a restricted path, though less common with dialogs) would ideally be caught and communicated, perhaps via the `status_message` or a dedicated error dialog if critical.

### Configuration and State Persistence Errors

*   **Saving/Loading `IpaBuilderApp` State**:
    *   Serialization (`serde_json::to_string`) or deserialization (`serde_json::from_str`) can fail.
    *   `eframe`'s storage mechanism (`storage.set_string`, `storage.get_string`) might also encounter issues, though `eframe` often handles these internally.
    *   These errors are logged using `log::error!`.
    *   If loading fails, the application typically falls back to a default state (`IpaBuilderApp::default()`) to ensure it can still start.
*   **Saving Metrics (`MetricsCollector::save_metrics_to_file`)**:
    *   File I/O errors (opening file, writing to file) are caught.
    *   Errors are logged (`log::error!`). The failure to save metrics is generally not critical to the app's core functionality, so it doesn't usually halt the application or show a UI error, but logging is important for diagnostics.

### Logging

*   The `log` crate facade, along with an implementation like `env_logger`, is used throughout the application.
*   `log::error!("...")` is used for critical errors.
*   `log::warn!("...")` for non-critical issues or potential problems.
*   `log::info!("...")` for general operational information.
*   `log::debug!("...")` and `log::trace!("...")` for detailed debugging information, usually enabled only in debug builds or via environment variables (e.g., `RUST_LOG=info` or `RUST_LOG=ipa_builder=debug`).

### General Principles

*   **User Feedback**: Prioritize clear, user-understandable messages for errors that directly impact their actions (e.g., IPA generation failure).
*   **Graceful Degradation**: If a non-critical component fails (like metrics saving), the application should continue to function where possible.
*   **Logging**: Comprehensive logging helps developers diagnose issues that users might report or that occur silently.

By combining these approaches, IPA Builder aims to be resilient and provide helpful feedback when errors occur.

*(More sections will be filled in subsequently)*
