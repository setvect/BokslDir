// Data Models
pub mod file_entry;
pub mod operation;
pub mod panel_state;
pub mod tab_state;

// Phase 2.2+에서 사용 예정
#[allow(unused_imports)]
pub use file_entry::{FileEntry, FileType};
pub use panel_state::PanelState;
pub use tab_state::PanelTabs;
// Phase 4에서 사용 예정
#[allow(unused_imports)]
pub use panel_state::{SortBy, SortOrder};
// Phase 3.2: 파일 작업 모델 (app.rs에서 직접 import하므로 re-export 불필요)
