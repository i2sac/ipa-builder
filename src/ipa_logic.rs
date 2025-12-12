use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::result::ZipError;
use zip::write::FileOptions;
use walkdir::WalkDir;
use tempfile::tempdir;
use thiserror::Error;

use crate::app::AppConfig;

#[derive(Error, Debug)]
pub enum IpaError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] ZipError),
    #[error("WalkDir error: {0}")]
    WalkDir(#[from] walkdir::Error),
    #[error("Temporary directory creation failed: {0}")]
    TempDir(std::io::Error),
    #[error("Input file '{0}' not found")]
    InputFileNotFound(PathBuf),
    #[error("Output directory '{0}' not found or is not a directory")]
    OutputDirectoryInvalid(PathBuf),
    #[error("The structure of the zip file is not as expected. Could not find a top-level .app directory or a nested one.")]
    UnexpectedZipStructure(PathBuf),
    #[error("Failed to create Payload directory at {0}")]
    PayloadCreationFailed(PathBuf),
    #[error("Failed to move/copy .app bundle to Payload directory: {0}")]
    MoveToPayloadFailed(PathBuf),
    #[error("Final IPA file name is invalid: {0}")]
    InvalidIpaName(String),
    #[error("Generated IPA has invalid structure: {0}")]
    InvalidIpaStructure(String),
}


/// Generates an IPA file from a Runner.app.zip file.
///
/// Steps:
/// 1. Create a temporary directory.
/// 2. Extract the input `Runner.app.zip` into the temporary directory.
/// 3. Locate the `.app` bundle (it might be nested, e.g., `SomeFolder/Runner.app` or just `Runner.app`).
/// 4. Create a `Payload` directory in a new temporary location for IPA creation.
/// 5. Move/copy the found `.app` bundle into this `Payload` directory.
/// 6. Compress the `Payload` directory into a new .zip file.
/// 7. Rename this .zip file to `app_name.ipa` and save it to the `output_directory`.
pub fn generate_ipa(config: &AppConfig, output_dir: &Path) -> Result<PathBuf, IpaError> {
    log::info!("Starting IPA generation for '{}' from '{}'", config.app_name, std::path::Path::new(&config.input_zip_path).display());

    if !std::path::Path::new(&config.input_zip_path).exists() {
        return Err(IpaError::InputFileNotFound(config.input_zip_path.clone().into()));
    }
    if !output_dir.is_dir() {
        return Err(IpaError::OutputDirectoryInvalid(output_dir.to_path_buf()));
    }

    // 1. Create a temporary directory for extraction
    let extract_temp_dir = tempdir().map_err(IpaError::TempDir)?;
    log::debug!("Created extraction temp dir: {}", extract_temp_dir.path().display());

    // 2. Extract the input Runner.app.zip
    let input_file = File::open(&config.input_zip_path)?;
    let mut archive = zip::ZipArchive::new(input_file)?;
    archive.extract(extract_temp_dir.path())?;
    log::info!("Extracted '{}' to '{}'", std::path::Path::new(&config.input_zip_path).file_name().unwrap_or_default().to_string_lossy(), extract_temp_dir.path().display());

    // 3. Locate the .app bundle
    let mut app_bundle_path: Option<PathBuf> = None;
    for entry_result in WalkDir::new(extract_temp_dir.path()).min_depth(1).max_depth(3) { // Increased max_depth slightly
        let entry = entry_result?;
        let path = entry.path();
        if path.is_dir() && path.extension().map_or(false, |ext| ext == "app") {
            if path.join("Info.plist").exists() { // A good indicator of an app bundle
                log::info!("Found candidate .app bundle: {}", path.display());
                app_bundle_path = Some(path.to_path_buf());
                break; 
            }
        }
    }
    
    let app_bundle_to_payload = app_bundle_path.ok_or_else(|| IpaError::UnexpectedZipStructure(extract_temp_dir.path().to_path_buf()))?;
    log::info!("Identified app bundle to be packaged: {}", app_bundle_to_payload.display());

    // 4. Create a `Payload` directory in a new temporary location for IPA creation.
    let ipa_build_temp_dir = tempdir().map_err(IpaError::TempDir)?;
    let payload_dir = ipa_build_temp_dir.path().join("Payload");
    fs::create_dir_all(&payload_dir).map_err(|_e| IpaError::PayloadCreationFailed(payload_dir.clone()))?;
    log::debug!("Created Payload directory: {}", payload_dir.display());

    // 5. Copy the found `.app` bundle into this `Payload` directory.
    let dest_app_path_in_payload = payload_dir.join(app_bundle_to_payload.file_name().unwrap_or_else(|| std::ffi::OsStr::new("Runner.app")));
    
    copy_dir_all(&app_bundle_to_payload, &dest_app_path_in_payload)
        .map_err(|e| {
            log::error!("Failed to copy {} to {}: {}", app_bundle_to_payload.display(), dest_app_path_in_payload.display(), e);
            IpaError::MoveToPayloadFailed(dest_app_path_in_payload.clone())
        })?;
    log::info!("Copied '{}' to '{}'", app_bundle_to_payload.file_name().unwrap_or_default().to_string_lossy(), dest_app_path_in_payload.display());

    // 6. Compress the `Payload` directory into a new .zip file.
    let ipa_file_name_str = config.output_ipa_name.trim().to_string();
    if ipa_file_name_str.is_empty() || !ipa_file_name_str.to_lowercase().ends_with(".ipa") {
        return Err(IpaError::InvalidIpaName(ipa_file_name_str));
    }
    if ipa_file_name_str.contains('/') || ipa_file_name_str.contains('\\') {
        return Err(IpaError::InvalidIpaName(ipa_file_name_str));
    }
    let final_ipa_path = output_dir.join(&ipa_file_name_str);
    let ipa_file = File::create(&final_ipa_path)?;
    let mut zip_writer = zip::ZipWriter::new(ipa_file);
    let dir_options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);
    let file_options_default = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    log::info!("Starting compression of Payload directory to {}", final_ipa_path.display());
    let walkdir_base = ipa_build_temp_dir.path(); // Base for stripping prefix
    let mut buffer = Vec::new();

    for entry_result in WalkDir::new(&payload_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry_result.path();
        // Path in zip should be relative to *inside* ipa_build_temp_dir, e.g., "Payload/AppName.app/file"
        let name_in_zip = path.strip_prefix(walkdir_base).unwrap(); 

        let zip_entry_name = zip_name_from_relative_path(name_in_zip, path.is_dir());
        if zip_entry_name.is_empty() {
            continue;
        }

        if path.is_file() {
            let mut f = File::open(path)?;
            f.read_to_end(&mut buffer)?;

            let perm = unix_permissions_for_payload_file(path, &buffer);
            let file_options = file_options_default.unix_permissions(perm);

            log::trace!("Adding file to zip: {:?} as {}", path, zip_entry_name);
            zip_writer.start_file(zip_entry_name, file_options)?;
            zip_writer.write_all(&buffer)?;
            buffer.clear();
        } else {
            log::trace!("Adding directory to zip: {:?} as {}", path, zip_entry_name);
            zip_writer.add_directory(zip_entry_name, dir_options)?;
        }
    }
    zip_writer.finish()?;
    log::info!("Successfully created IPA: {}", final_ipa_path.display());

    validate_generated_ipa(&final_ipa_path)?;

    Ok(final_ipa_path)
}

fn validate_generated_ipa(ipa_path: &Path) -> Result<(), IpaError> {
    let ipa_file = File::open(ipa_path)?;
    let mut archive = zip::ZipArchive::new(ipa_file)?;

    let mut found_plist = false;
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let name = file.name();

        if name.starts_with("Payload/") && name.ends_with(".app/Info.plist") {
            found_plist = true;
            break;
        }
    }

    if !found_plist {
        return Err(IpaError::InvalidIpaStructure(
            "Missing Payload/<App>.app/Info.plist".to_string(),
        ));
    }

    Ok(())
}

fn zip_name_from_relative_path(relative_path: &Path, is_dir: bool) -> String {
    let mut s = relative_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");

    if is_dir {
        if !s.is_empty() && !s.ends_with('/') {
            s.push('/');
        }
    }

    s
}

fn unix_permissions_for_payload_file(file_path: &Path, file_bytes: &[u8]) -> u32 {
    if is_macho(file_bytes) {
        return 0o755;
    }
    if matches!(file_path.extension().and_then(|e| e.to_str()), Some("dylib")) {
        return 0o755;
    }
    0o644
}

fn is_macho(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    matches!(
        magic,
        0xFEEDFACE
            | 0xFEEDFACF
            | 0xCAFEBABE
            | 0xBEBAFECA
            | 0xCEFAEDFE
            | 0xCFFAEDFE
    )
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(dst.as_ref())?;
    for entry_result in fs::read_dir(src.as_ref())? {
        let entry = entry_result?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.as_ref().join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use zip::write::FileOptions;
    use uuid::Uuid;
    use chrono::Utc;

    // Helper to create a mock .app bundle structure within a directory
    fn create_mock_app_bundle(app_dir: &Path, app_name: &str) -> std::io::Result<()> {
        fs::create_dir_all(app_dir)?;
        File::create(app_dir.join("Info.plist"))?.write_all(b"Mock Info.plist")?;
        File::create(app_dir.join(app_name))?.write_all(b"Mock executable")?;
        Ok(())
    }

    // Helper to create a mock zip file containing a .app bundle
    fn create_mock_app_zip(zip_path: &Path, app_bundle_name: &str, internal_path_prefix: Option<&str>) -> std::io::Result<()> {
        let temp_source_dir = tempdir().unwrap();
        let app_bundle_source_path = if let Some(prefix) = internal_path_prefix {
            temp_source_dir.path().join(prefix).join(format!("{}.app", app_bundle_name))
        } else {
            temp_source_dir.path().join(format!("{}.app", app_bundle_name))
        };
        create_mock_app_bundle(&app_bundle_source_path, app_bundle_name)?;

        let file = File::create(zip_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        let walkdir = WalkDir::new(temp_source_dir.path());
        let mut buffer = Vec::new();

        for entry_result in walkdir.into_iter().filter_map(|e| e.ok()) {
            let path = entry_result.path();
            let name = path.strip_prefix(temp_source_dir.path()).unwrap();

            if path.is_file() {
                zip.start_file(name.to_string_lossy().into_owned(), options)?;
                let mut f = File::open(path)?;
                f.read_to_end(&mut buffer)?;
                zip.write_all(&buffer)?;
                buffer.clear();
            } else if !name.as_os_str().is_empty() {
                zip.add_directory(name.to_string_lossy().into_owned(), options)?;
            }
        }
        zip.finish()?;
        Ok(())
    }

    #[test]
    fn test_simple_ipa_generation_runner_app() {
        let temp_root = tempdir().unwrap();
        let input_dir = temp_root.path().join("input");
        let output_dir = temp_root.path().join("output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let mock_zip_path = input_dir.join("TestRunner.app.zip");
        create_mock_app_zip(&mock_zip_path, "Runner", None).unwrap(); // Creates Runner.app at root of zip

        let app_name = "MyTestApp".to_string();
        let config = AppConfig {
            id: Uuid::new_v4().to_string(),
            input_zip_path: mock_zip_path.to_string_lossy().into_owned(),
            app_name: app_name.clone(),
            output_ipa_name: format!("{}.ipa", app_name),
            created_at: Utc::now(),
            last_generated_at: None,
        };

        let result = generate_ipa(&config, &output_dir);
        assert!(result.is_ok(), "generate_ipa failed: {:?}", result.err());

        let output_ipa_path = output_dir.join("MyTestApp.ipa");
        assert!(output_ipa_path.exists(), "Output IPA file was not created.");

        let ipa_file = File::open(output_ipa_path).unwrap();
        let mut archive = zip::ZipArchive::new(ipa_file).unwrap();
        assert!(archive.by_name("Payload/Runner.app/Info.plist").is_ok());
        assert!(archive.by_name("Payload/Runner.app/Runner").is_ok());
    }

    #[test]
    fn test_nested_app_bundle_generation() {
        let temp_root = tempdir().unwrap();
        let input_dir = temp_root.path().join("input_nested");
        let output_dir = temp_root.path().join("output_nested");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let mock_zip_path = input_dir.join("MyProject.app.zip");
        // Creates a zip with SomeFolder/MyProject.app
        create_mock_app_zip(&mock_zip_path, "MyProject", Some("SomeFolder")).unwrap(); 

        let app_name = "NestedAppTest".to_string();
        let config = AppConfig {
            id: Uuid::new_v4().to_string(),
            input_zip_path: mock_zip_path.to_string_lossy().into_owned(),
            app_name: app_name.clone(),
            output_ipa_name: format!("{}.ipa", app_name),
            created_at: Utc::now(),
            last_generated_at: None,
        };

        let result = generate_ipa(&config, &output_dir);
        assert!(result.is_ok(), "generate_ipa for nested failed: {:?}", result.err());

        let output_ipa_path = output_dir.join("NestedAppTest.ipa");
        assert!(output_ipa_path.exists(), "Output IPA file for nested was not created.");

        let ipa_file = File::open(output_ipa_path).unwrap();
        let mut archive = zip::ZipArchive::new(ipa_file).unwrap();
        assert!(archive.by_name("Payload/MyProject.app/Info.plist").is_ok());
        assert!(archive.by_name("Payload/MyProject.app/MyProject").is_ok());
    }

     #[test]
    fn test_input_file_not_found() {
        let temp_root = tempdir().unwrap();
        let output_dir = temp_root.path().join("output_notfound");
        fs::create_dir_all(&output_dir).unwrap();

        let app_name = "NotFoundTest".to_string();
        let config = AppConfig {
            id: Uuid::new_v4().to_string(),
            input_zip_path: PathBuf::from("non_existent_file.zip").to_string_lossy().into_owned(),
            app_name: app_name.clone(),
            output_ipa_name: format!("{}.ipa", app_name),
            created_at: Utc::now(),
            last_generated_at: None,
        };

        let result = generate_ipa(&config, &output_dir);
        assert!(matches!(result, Err(IpaError::InputFileNotFound(_))));
    }

    #[test]
    fn test_app_bundle_not_found_in_zip() {
        let temp_root = tempdir().unwrap();
        let input_dir = temp_root.path().join("input_no_app");
        let output_dir = temp_root.path().join("output_no_app");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let mock_zip_path = input_dir.join("Empty.zip");
        // Create an empty zip or a zip without a .app directory
        let file = File::create(&mock_zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("readme.txt", FileOptions::default()).unwrap();
        zip.write_all(b"empty").unwrap();
        zip.finish().unwrap();

        let app_name = "NoAppBundleTest".to_string();
        let config = AppConfig {
            id: Uuid::new_v4().to_string(),
            input_zip_path: mock_zip_path.to_string_lossy().into_owned(),
            app_name: app_name.clone(),
            output_ipa_name: format!("{}.ipa", app_name),
            created_at: Utc::now(),
            last_generated_at: None,
        };

        let result = generate_ipa(&config, &output_dir);
        assert!(matches!(result, Err(IpaError::UnexpectedZipStructure(_))));
    }
}

