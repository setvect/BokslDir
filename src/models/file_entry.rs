#![allow(dead_code)]

use std::fs::Permissions;
use std::path::PathBuf;
use std::time::SystemTime;

/// 파일 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// 디렉토리
    Directory,
    /// 일반 파일
    File,
    /// 심볼릭 링크
    Symlink,
    /// 실행 파일
    Executable,
}

/// 파일 엔트리
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// 파일/디렉토리 이름
    pub name: String,
    /// 전체 경로
    pub path: PathBuf,
    /// 파일 타입
    pub file_type: FileType,
    /// 바이트 단위 크기
    pub size: u64,
    /// 수정 시간
    pub modified: SystemTime,
    /// 권한 (Unix 계열)
    pub permissions: Option<Permissions>,
    /// 숨김 파일 여부
    pub is_hidden: bool,
}

impl FileEntry {
    /// 새 파일 엔트리 생성
    pub fn new(
        name: String,
        path: PathBuf,
        file_type: FileType,
        size: u64,
        modified: SystemTime,
        permissions: Option<Permissions>,
        is_hidden: bool,
    ) -> Self {
        Self {
            name,
            path,
            file_type,
            size,
            modified,
            permissions,
            is_hidden,
        }
    }

    /// 디렉토리 여부 확인
    pub fn is_directory(&self) -> bool {
        self.file_type == FileType::Directory
    }

    /// 파일 여부 확인
    pub fn is_file(&self) -> bool {
        matches!(self.file_type, FileType::File | FileType::Executable)
    }

    /// 심볼릭 링크 여부 확인
    pub fn is_symlink(&self) -> bool {
        self.file_type == FileType::Symlink
    }

    /// 실행 파일 여부 확인
    pub fn is_executable(&self) -> bool {
        self.file_type == FileType::Executable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_entry_creation() {
        let entry = FileEntry::new(
            "test.txt".to_string(),
            PathBuf::from("/tmp/test.txt"),
            FileType::File,
            1024,
            SystemTime::now(),
            None,
            false,
        );

        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.file_type, FileType::File);
        assert_eq!(entry.size, 1024);
        assert!(!entry.is_hidden);
    }

    #[test]
    fn test_file_type_checks() {
        let dir_entry = FileEntry::new(
            "dir".to_string(),
            PathBuf::from("/tmp/dir"),
            FileType::Directory,
            0,
            SystemTime::now(),
            None,
            false,
        );

        assert!(dir_entry.is_directory());
        assert!(!dir_entry.is_file());

        let file_entry = FileEntry::new(
            "file.txt".to_string(),
            PathBuf::from("/tmp/file.txt"),
            FileType::File,
            100,
            SystemTime::now(),
            None,
            false,
        );

        assert!(!file_entry.is_directory());
        assert!(file_entry.is_file());
    }
}
