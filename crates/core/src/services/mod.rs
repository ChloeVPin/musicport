//! high-level ops over device+db - what the ui/cli calls
//! mutating ops: backup, download db, edit locally, upload files + db, clear sidecars - no reboot

mod library;
mod naming;
mod phone;
pub mod reports;

pub use phone::Phone;
pub use reports::{AddReport, ExportReport, RemoveReport};