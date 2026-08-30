//! tauri commands -> core
//! each one opens a fresh Phone so Device/Afc dont need to be Send - only state is picked device
//! errors just become strings for the frontend

use std::path::PathBuf;
use std::sync::Mutex;

use musicport_core::db::TrackRow;
use musicport_core::device::{DeviceInfo, DeviceListing};
use musicport_core::services::{AddReport, ExportReport, Phone, RemoveReport};
use tauri::State;

type CmdError = String;

/// app state - just the picked device
pub struct AppState {
    /// UDID of the currently selected device, if any.
    pub udid: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            udid: Mutex::new(None),
        }
    }
}

/// get picked phone or first found
fn selected_phone(state: &AppState) -> Result<Phone, CmdError> {
    let udid = state.udid.lock().unwrap().clone();
    Phone::connect(udid.as_deref()).map_err(|e| format!("{e:#}"))
}

/// Connect to a device and remember it for future commands.
#[tauri::command]
pub fn connect(state: State<'_, AppState>, udid: Option<String>) -> Result<DeviceInfo, CmdError> {
    let phone = Phone::connect(udid.as_deref()).map_err(|e| format!("{e:#}"))?;
    let info = phone.info().map_err(|e| format!("{e:#}"))?;
    *state.udid.lock().unwrap() = Some(info.udid.clone());
    Ok(info)
}

/// Devices currently visible over USB / Wi-Fi (no connection made).
#[tauri::command]
pub fn list_devices() -> Result<Vec<DeviceListing>, CmdError> {
    Phone::discover().map_err(|e| format!("{e:#}"))
}

/// list tracks, optional search filter
#[tauri::command]
pub fn list_tracks(state: State<'_, AppState>, query: Option<String>) -> Result<Vec<TrackRow>, CmdError> {
    selected_phone(state.inner())?
        .list_tracks(query.as_deref())
        .map_err(|e| format!("{e:#}"))
}

/// Add audio files (local absolute paths) to the device library.
#[tauri::command]
pub fn add_files(
    state: State<'_, AppState>,
    files: Vec<String>,
    force: bool,
) -> Result<AddReport, CmdError> {
    let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    selected_phone(state.inner())?
        .add_files(&paths, force)
        .map_err(|e| format!("{e:#}"))
}

/// Remove tracks from the device library by pid.
#[tauri::command]
pub fn remove_tracks(
    state: State<'_, AppState>,
    pids: Vec<i64>,
) -> Result<RemoveReport, CmdError> {
    selected_phone(state.inner())?
        .remove_tracks(&pids)
        .map_err(|e| format!("{e:#}"))
}

/// Export matching tracks into a local directory.
#[tauri::command]
pub fn export_tracks(
    state: State<'_, AppState>,
    query: Option<String>,
    out_dir: String,
) -> Result<ExportReport, CmdError> {
    let dir = PathBuf::from(&out_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{e:#}"))?;
    selected_phone(state.inner())?
        .export_tracks(query.as_deref(), &dir)
        .map_err(|e| format!("{e:#}"))
}
