pub mod url_convert;
pub mod url;
pub mod project;
mod session_signal_extract;
pub mod session_transcript;
pub mod session_insights;
pub mod session_log;

pub use url::{ingest_url, IngestResult, IngestUrlOpts};
pub use project::{ingest_project, ProjectIngestResult};
pub use session_transcript::{build_signals, format_session_body, normalize_turns, parse_hook_input, parse_turns, HookInput, SessionSignals};
pub use session_insights::{extract_insights, InsightCandidate};
pub use session_log::{format_session_log, FleetMetrics, SessionLogInput, METRICS_VERSION};
