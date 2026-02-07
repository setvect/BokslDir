#![allow(dead_code)]

use crate::models::file_entry::{FileEntry, FileType};
use crate::utils::error::{BokslDirError, Result};
use std::fs::{self, Metadata};
use std::path::Path;

/// 파일 시스템 모듈
pub struct FileSystem;

impl FileSystem {
    /// 새 파일 시스템 인스턴스 생성
    pub fn new() -> Self {
        Self
    }

    /// 디렉토리 읽기
    ///
    /// 주어진 경로의 디렉토리를 읽어서 파일 엔트리 리스트를 반환합니다.
    pub fn read_directory(&self, path: &Path) -> Result<Vec<FileEntry>> {
        // 1. 경로 존재 확인
        if !path.exists() {
            return Err(BokslDirError::PathNotFound {
                path: path.to_path_buf(),
            });
        }

        // 2. 디렉토리 여부 확인
        if !path.is_dir() {
            return Err(BokslDirError::NotADirectory {
                path: path.to_path_buf(),
            });
        }

        // 3. 디렉토리 읽기
        let read_dir = fs::read_dir(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                BokslDirError::PermissionDenied {
                    path: path.to_path_buf(),
                }
            } else {
                BokslDirError::Io(e)
            }
        })?;

        // 4. 각 엔트리에 대해 메타데이터 파싱
        let mut entries = Vec::new();

        for entry in read_dir {
            // 에러 발생 시 해당 엔트리는 스킵
            let Ok(entry) = entry else { continue };

            let entry_path = entry.path();

            // 메타데이터 가져오기 (심볼릭 링크는 symlink_metadata 사용)
            let Ok(metadata) = entry.metadata() else {
                continue;
            };

            // 파일 이름
            let name = entry.file_name().to_string_lossy().to_string();

            // 파일 타입 판단
            let file_type = self.get_file_type(&entry_path, &metadata);

            // 크기 (디렉토리는 0)
            let size = if metadata.is_dir() { 0 } else { metadata.len() };

            // 수정 시간
            let modified = metadata
                .modified()
                .unwrap_or_else(|_| std::time::SystemTime::now());

            // 권한 (Unix 계열에서만)
            let permissions = Some(metadata.permissions());

            // 숨김 파일 여부
            let is_hidden = self.is_hidden(&entry_path);

            entries.push(FileEntry::new(
                name,
                entry_path,
                file_type,
                size,
                modified,
                permissions,
                is_hidden,
            ));
        }

        Ok(entries)
    }

    /// 파일 타입 판단
    #[allow(clippy::unused_self)]
    fn get_file_type(&self, _path: &Path, metadata: &Metadata) -> FileType {
        // 1. 디렉토리 확인
        if metadata.is_dir() {
            return FileType::Directory;
        }

        // 2. 심볼릭 링크 확인
        if metadata.is_symlink() {
            return FileType::Symlink;
        }

        // 3. 실행 파일 확인 (Unix 계열)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            // 실행 권한이 있는지 확인 (owner, group, other 중 하나라도)
            if mode & 0o111 != 0 {
                return FileType::Executable;
            }
        }

        // 4. 일반 파일
        FileType::File
    }

    /// 숨김 파일 여부 판단
    #[allow(clippy::unused_self)]
    fn is_hidden(&self, path: &Path) -> bool {
        // 파일명 가져오기
        let file_name = match path.file_name() {
            Some(name) => name.to_string_lossy(),
            None => return false,
        };

        // Unix: '.'으로 시작하는 파일
        #[cfg(unix)]
        {
            return file_name.starts_with('.');
        }

        // Windows: 파일 속성의 HIDDEN 플래그 확인
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if let Ok(metadata) = path.metadata() {
                const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
                return (metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN) != 0;
            }
        }

        // 기본적으로 '.'으로 시작하면 숨김 파일로 간주
        #[allow(unreachable_code)]
        file_name.starts_with('.')
    }

    /// 경로 존재 확인
    #[allow(clippy::unused_self)]
    pub fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    /// 디렉토리 여부 확인
    #[allow(clippy::unused_self)]
    pub fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    // === Phase 3.2: 파일 복사/이동 메서드 ===

    /// 파일 복사
    ///
    /// 소스 파일을 대상 경로로 복사합니다.
    /// 반환값: 복사된 바이트 수
    #[allow(clippy::unused_self)]
    pub fn copy_file(&self, src: &Path, dest: &Path) -> Result<u64> {
        // 소스와 대상이 동일한지 확인
        if src == dest {
            return Err(BokslDirError::SameSourceAndDest {
                path: src.to_path_buf(),
            });
        }

        // 소스 파일 존재 확인
        if !src.exists() {
            return Err(BokslDirError::PathNotFound {
                path: src.to_path_buf(),
            });
        }

        // 복사 실행
        fs::copy(src, dest).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                BokslDirError::PermissionDenied {
                    path: dest.to_path_buf(),
                }
            } else {
                BokslDirError::CopyFailed {
                    src: src.to_path_buf(),
                    dest: dest.to_path_buf(),
                    reason: e.to_string(),
                }
            }
        })
    }

    /// 디렉토리 재귀 복사
    ///
    /// 소스 디렉토리를 대상 경로로 재귀적으로 복사합니다.
    /// 반환값: 복사된 총 바이트 수
    pub fn copy_directory(&self, src: &Path, dest: &Path) -> Result<u64> {
        // 소스와 대상이 동일한지 확인
        if src == dest {
            return Err(BokslDirError::SameSourceAndDest {
                path: src.to_path_buf(),
            });
        }

        // 소스 디렉토리 존재 확인
        if !src.exists() {
            return Err(BokslDirError::PathNotFound {
                path: src.to_path_buf(),
            });
        }

        if !src.is_dir() {
            return Err(BokslDirError::NotADirectory {
                path: src.to_path_buf(),
            });
        }

        // 대상 디렉토리 생성
        fs::create_dir_all(dest).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                BokslDirError::PermissionDenied {
                    path: dest.to_path_buf(),
                }
            } else {
                BokslDirError::CopyFailed {
                    src: src.to_path_buf(),
                    dest: dest.to_path_buf(),
                    reason: e.to_string(),
                }
            }
        })?;

        let mut total_bytes = 0u64;

        // 소스 디렉토리 내용 순회
        for entry in fs::read_dir(src).map_err(BokslDirError::Io)? {
            let entry = entry.map_err(BokslDirError::Io)?;
            let entry_path = entry.path();
            let file_name = entry.file_name();
            let dest_path = dest.join(&file_name);

            if entry_path.is_dir() {
                // 재귀적으로 서브디렉토리 복사
                total_bytes += self.copy_directory(&entry_path, &dest_path)?;
            } else {
                // 파일 복사
                total_bytes += self.copy_file(&entry_path, &dest_path)?;
            }
        }

        Ok(total_bytes)
    }

    /// 파일 이동
    ///
    /// 소스 파일을 대상 경로로 이동합니다.
    /// 먼저 rename을 시도하고, 실패하면 복사 후 삭제합니다.
    /// 반환값: 이동된 바이트 수
    #[allow(clippy::unused_self)]
    pub fn move_file(&self, src: &Path, dest: &Path) -> Result<u64> {
        // 소스와 대상이 동일한지 확인
        if src == dest {
            return Err(BokslDirError::SameSourceAndDest {
                path: src.to_path_buf(),
            });
        }

        // 소스 파일 존재 확인
        if !src.exists() {
            return Err(BokslDirError::PathNotFound {
                path: src.to_path_buf(),
            });
        }

        // 파일 크기 미리 저장
        let file_size = src.metadata().map(|m| m.len()).unwrap_or(0);

        // 먼저 rename 시도 (같은 파일시스템 내에서는 빠름)
        if fs::rename(src, dest).is_ok() {
            return Ok(file_size);
        }

        // rename 실패 시 복사 후 삭제
        self.copy_file(src, dest)?;
        fs::remove_file(src).map_err(|e| BokslDirError::MoveFailed {
            src: src.to_path_buf(),
            dest: dest.to_path_buf(),
            reason: format!("Failed to remove source after copy: {}", e),
        })?;

        Ok(file_size)
    }

    /// 디렉토리 이동
    ///
    /// 소스 디렉토리를 대상 경로로 이동합니다.
    /// 먼저 rename을 시도하고, 실패하면 복사 후 삭제합니다.
    /// 반환값: 이동된 총 바이트 수
    pub fn move_directory(&self, src: &Path, dest: &Path) -> Result<u64> {
        // 소스와 대상이 동일한지 확인
        if src == dest {
            return Err(BokslDirError::SameSourceAndDest {
                path: src.to_path_buf(),
            });
        }

        // 소스 디렉토리 존재 확인
        if !src.exists() {
            return Err(BokslDirError::PathNotFound {
                path: src.to_path_buf(),
            });
        }

        if !src.is_dir() {
            return Err(BokslDirError::NotADirectory {
                path: src.to_path_buf(),
            });
        }

        // 전체 크기 미리 계산
        let (total_bytes, _) = self.calculate_total_size(&[src.to_path_buf()])?;

        // 먼저 rename 시도 (같은 파일시스템 내에서는 빠름)
        if fs::rename(src, dest).is_ok() {
            return Ok(total_bytes);
        }

        // rename 실패 시 복사 후 삭제
        self.copy_directory(src, dest)?;
        fs::remove_dir_all(src).map_err(|e| BokslDirError::MoveFailed {
            src: src.to_path_buf(),
            dest: dest.to_path_buf(),
            reason: format!("Failed to remove source after copy: {}", e),
        })?;

        Ok(total_bytes)
    }

    /// 경로 목록의 총 크기와 파일 수 계산
    ///
    /// 반환값: (총 바이트, 총 파일 수)
    pub fn calculate_total_size(&self, paths: &[std::path::PathBuf]) -> Result<(u64, usize)> {
        let mut total_bytes = 0u64;
        let mut total_files = 0usize;

        for path in paths {
            if path.is_file() {
                total_bytes += path.metadata().map(|m| m.len()).unwrap_or(0);
                total_files += 1;
            } else if path.is_dir() {
                let (bytes, files) = self.calculate_directory_size(path)?;
                total_bytes += bytes;
                total_files += files;
            }
        }

        Ok((total_bytes, total_files))
    }

    /// 디렉토리의 총 크기와 파일 수 계산 (재귀)
    fn calculate_directory_size(&self, path: &Path) -> Result<(u64, usize)> {
        let mut total_bytes = 0u64;
        let mut total_files = 0usize;

        for entry in fs::read_dir(path).map_err(BokslDirError::Io)? {
            let entry = entry.map_err(BokslDirError::Io)?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                total_bytes += entry_path.metadata().map(|m| m.len()).unwrap_or(0);
                total_files += 1;
            } else if entry_path.is_dir() {
                let (bytes, files) = self.calculate_directory_size(&entry_path)?;
                total_bytes += bytes;
                total_files += files;
            }
        }

        Ok((total_bytes, total_files))
    }

    /// 파일/디렉토리 존재 여부 확인
    #[allow(clippy::unused_self)]
    pub fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    /// 소스 목록을 평탄화하여 개별 파일 목록 생성
    ///
    /// 디렉토리는 재귀적으로 탐색하여 모든 파일을 포함합니다.
    /// 반환값: Vec<(source_path, dest_path, size)>
    pub fn flatten_sources(
        &self,
        sources: &[std::path::PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<(std::path::PathBuf, std::path::PathBuf, u64)>> {
        let mut result = Vec::new();

        for source in sources {
            let file_name = source.file_name().unwrap_or_default();
            let dest_base = dest_dir.join(file_name);

            if source.is_file() {
                let size = source.metadata().map(|m| m.len()).unwrap_or(0);
                result.push((source.clone(), dest_base, size));
            } else if source.is_dir() {
                self.flatten_directory(source, source, &dest_base, &mut result)?;
            }
        }

        Ok(result)
    }

    /// 디렉토리를 재귀적으로 평탄화
    fn flatten_directory(
        &self,
        base_source: &Path,
        current_source: &Path,
        dest_base: &Path,
        result: &mut Vec<(std::path::PathBuf, std::path::PathBuf, u64)>,
    ) -> Result<()> {
        for entry in fs::read_dir(current_source).map_err(BokslDirError::Io)? {
            let entry = entry.map_err(BokslDirError::Io)?;
            let entry_path = entry.path();

            // 상대 경로 계산
            let relative = entry_path.strip_prefix(base_source).unwrap_or(&entry_path);
            let dest_path = dest_base.join(relative);

            if entry_path.is_file() {
                let size = entry_path.metadata().map(|m| m.len()).unwrap_or(0);
                result.push((entry_path, dest_path, size));
            } else if entry_path.is_dir() {
                self.flatten_directory(base_source, &entry_path, dest_base, result)?;
            }
        }

        Ok(())
    }
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn test_filesystem_creation() {
        let fs = FileSystem::new();
        assert!(fs.exists(&PathBuf::from(".")));
    }

    #[test]
    fn test_read_directory() {
        let fs = FileSystem::new();

        // 현재 디렉토리 읽기 테스트
        let current_dir = std::env::current_dir().unwrap();
        let result = fs.read_directory(&current_dir);

        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_read_nonexistent_directory() {
        let fs = FileSystem::new();
        let result = fs.read_directory(&PathBuf::from("/nonexistent/path/12345"));

        assert!(result.is_err());
        match result {
            Err(BokslDirError::PathNotFound { .. }) => {}
            _ => panic!("Expected PathNotFound error"),
        }
    }

    #[test]
    fn test_is_hidden() {
        let fs = FileSystem::new();

        // Unix: '.'으로 시작하는 파일은 숨김 파일
        let hidden_path = PathBuf::from(".hidden");
        assert!(fs.is_hidden(&hidden_path));

        let visible_path = PathBuf::from("visible.txt");
        assert!(!fs.is_hidden(&visible_path));
    }

    #[test]
    fn test_file_type_detection() {
        let fs = FileSystem::new();

        // 임시 디렉토리 생성
        let temp_dir = std::env::temp_dir().join("boksldir_test");
        let _ = fs::create_dir_all(&temp_dir);

        // 임시 파일 생성
        let temp_file = temp_dir.join("test.txt");
        let mut file = File::create(&temp_file).unwrap();
        writeln!(file, "test content").unwrap();

        // 디렉토리 읽기
        let entries = fs.read_directory(&temp_dir).unwrap();

        // 파일 찾기
        let file_entry = entries.iter().find(|e| e.name == "test.txt");
        assert!(file_entry.is_some());

        let file_entry = file_entry.unwrap();
        assert_eq!(file_entry.file_type, FileType::File);
        assert!(!file_entry.is_directory());

        // 정리
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
