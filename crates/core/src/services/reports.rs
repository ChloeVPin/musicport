//! results from ops - serializable for the ui

/// Result of a `Phone::add_files` run.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct AddReport {
    pub added: usize,
    pub skipped: usize,
    pub pids: Vec<i64>,
    /// The `Fxx` bucket the new files were placed in.
    pub folder: String,
    pub messages: Vec<String>,
}

/// Result of a `Phone::remove_tracks` run.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct RemoveReport {
    pub removed: usize,
    pub messages: Vec<String>,
}

/// Result of a `Phone::export_tracks` run.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ExportReport {
    pub exported: usize,
    pub out_dir: String,
    pub messages: Vec<String>,
}