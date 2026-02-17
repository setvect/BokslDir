//! 다이얼로그 시스템 (Phase 3.2)
//!
//! 파일 복사/이동 작업에 필요한 다이얼로그 위젯 정의

#![allow(dead_code)]

mod builders;
mod kind;
mod render;

pub use kind::{DialogKind, InputPurpose};
pub use render::Dialog;
