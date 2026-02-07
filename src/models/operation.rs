//! 파일 작업 모델 (Phase 3.2)
//!
//! 파일 복사/이동 작업에 필요한 데이터 구조 정의

#![allow(dead_code)]

use std::path::PathBuf;

/// 평탄화된 파일 정보 (개별 파일 단위 처리용)
#[derive(Debug, Clone)]
pub struct FlattenedFile {
    /// 원본 파일 전체 경로
    pub source: PathBuf,
    /// 대상 파일 전체 경로
    pub dest: PathBuf,
    /// 파일 크기 (바이트)
    pub size: u64,
}

/// 작업 유형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    /// 복사
    Copy,
    /// 이동
    Move,
}

impl OperationType {
    /// 작업 유형 이름 반환
    pub fn name(&self) -> &'static str {
        match self {
            OperationType::Copy => "Copy",
            OperationType::Move => "Move",
        }
    }

    /// 한글 이름 반환
    pub fn name_ko(&self) -> &'static str {
        match self {
            OperationType::Copy => "복사",
            OperationType::Move => "이동",
        }
    }
}

/// 충돌 해결 방법
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// 덮어쓰기
    Overwrite,
    /// 건너뛰기
    Skip,
    /// 모두 덮어쓰기
    OverwriteAll,
    /// 모두 건너뛰기
    SkipAll,
    /// 취소
    Cancel,
}

/// 작업 진행 상태
#[derive(Debug, Clone)]
pub struct OperationProgress {
    /// 작업 유형
    pub operation_type: OperationType,
    /// 현재 처리 중인 파일
    pub current_file: String,
    /// 완료된 파일 수
    pub files_completed: usize,
    /// 전체 파일 수
    pub total_files: usize,
    /// 복사된 바이트 수
    pub bytes_copied: u64,
    /// 전체 바이트 수
    pub total_bytes: u64,
}

impl OperationProgress {
    /// 새 진행 상태 생성
    pub fn new(operation_type: OperationType, total_files: usize, total_bytes: u64) -> Self {
        Self {
            operation_type,
            current_file: String::new(),
            files_completed: 0,
            total_files,
            bytes_copied: 0,
            total_bytes,
        }
    }

    /// 진행률 계산 (0-100)
    pub fn percentage(&self) -> u8 {
        if self.total_bytes == 0 {
            if self.total_files == 0 {
                100
            } else {
                ((self.files_completed as f64 / self.total_files as f64) * 100.0) as u8
            }
        } else {
            ((self.bytes_copied as f64 / self.total_bytes as f64) * 100.0) as u8
        }
    }
}

/// 작업 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    /// 대기 중 (대상 경로 입력 대기)
    Pending,
    /// 처리 중
    Processing,
    /// 충돌 대기 (사용자 선택 대기)
    WaitingConflict,
    /// 완료
    Completed,
}

/// 대기 중인 작업
#[derive(Debug, Clone)]
pub struct PendingOperation {
    /// 작업 유형
    pub operation_type: OperationType,
    /// 소스 파일/디렉토리 목록 (원본 선택)
    pub sources: Vec<PathBuf>,
    /// 대상 디렉토리
    pub dest_dir: PathBuf,
    /// 평탄화된 파일 목록 (개별 파일 단위)
    pub flattened_files: Vec<FlattenedFile>,
    /// 충돌 해결 방법 (OverwriteAll/SkipAll 시 사용)
    pub conflict_resolution: Option<ConflictResolution>,
    /// 현재 처리 중인 인덱스 (flattened_files 인덱스)
    pub current_index: usize,
    /// 작업 상태
    pub state: OperationState,
    /// 진행 상태
    pub progress: OperationProgress,
    /// 누적 에러 목록
    pub errors: Vec<String>,
    /// 완료된 파일 수
    pub completed_count: usize,
}

impl PendingOperation {
    /// 새 대기 작업 생성
    pub fn new(operation_type: OperationType, sources: Vec<PathBuf>, dest_dir: PathBuf) -> Self {
        let total_files = sources.len();
        Self {
            operation_type,
            sources,
            dest_dir,
            flattened_files: Vec::new(),
            conflict_resolution: None,
            current_index: 0,
            state: OperationState::Pending,
            progress: OperationProgress::new(operation_type, total_files, 0),
            errors: Vec::new(),
            completed_count: 0,
        }
    }

    /// 진행 상태를 Processing으로 변경하고 전체 크기/파일 수 설정
    pub fn start_processing(&mut self, total_bytes: u64, total_files: usize) {
        self.state = OperationState::Processing;
        self.progress.total_bytes = total_bytes;
        self.progress.total_files = total_files;
    }

    /// 현재 파일 이름 업데이트
    pub fn set_current_file(&mut self, name: &str) {
        self.progress.current_file = name.to_string();
    }

    /// 파일/디렉토리 완료 시 진행 상태 업데이트
    ///
    /// `file_count`: 완료된 파일 수 (디렉토리의 경우 내부 파일 수)
    pub fn files_completed(&mut self, bytes: u64, file_count: usize) {
        self.progress.files_completed += file_count;
        self.progress.bytes_copied += bytes;
        self.completed_count += 1;
    }

    /// 파일/디렉토리 건너뛰기 (에러 또는 Skip)
    pub fn file_skipped(&mut self) {
        // 건너뛴 항목은 진행률에 반영하지 않음 (total에서 제외하는 것이 더 정확하지만 복잡)
        // 대신 완료 시 total_files까지 도달하지 않을 수 있음
    }

    /// 에러 추가
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// 모든 파일 처리 완료 여부
    pub fn is_all_processed(&self) -> bool {
        self.current_index >= self.flattened_files.len()
    }

    /// 평탄화된 파일 목록 설정
    pub fn set_flattened_files(&mut self, files: Vec<FlattenedFile>) {
        self.flattened_files = files;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_type_name() {
        assert_eq!(OperationType::Copy.name(), "Copy");
        assert_eq!(OperationType::Move.name(), "Move");
        assert_eq!(OperationType::Copy.name_ko(), "복사");
        assert_eq!(OperationType::Move.name_ko(), "이동");
    }

    #[test]
    fn test_operation_progress_percentage() {
        let mut progress = OperationProgress::new(OperationType::Copy, 10, 1000);

        // 초기 상태
        assert_eq!(progress.percentage(), 0);

        // 50% 진행
        progress.bytes_copied = 500;
        assert_eq!(progress.percentage(), 50);

        // 완료
        progress.bytes_copied = 1000;
        assert_eq!(progress.percentage(), 100);
    }

    #[test]
    fn test_operation_progress_percentage_zero_bytes() {
        let mut progress = OperationProgress::new(OperationType::Copy, 4, 0);

        // 바이트 기준이 없으면 파일 수 기준
        progress.files_completed = 2;
        assert_eq!(progress.percentage(), 50);
    }

    #[test]
    fn test_pending_operation() {
        let sources = vec![PathBuf::from("/tmp/file1"), PathBuf::from("/tmp/file2")];
        let dest = PathBuf::from("/home/user");
        let pending = PendingOperation::new(OperationType::Copy, sources.clone(), dest.clone());

        assert_eq!(pending.operation_type, OperationType::Copy);
        assert_eq!(pending.sources.len(), 2);
        assert_eq!(pending.dest_dir, dest);
        assert!(pending.conflict_resolution.is_none());
    }
}
