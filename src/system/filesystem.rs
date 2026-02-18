#![allow(dead_code)]

use crate::models::file_entry::{FileEntry, FileType};
use crate::models::operation::{FlattenedEntryKind, FlattenedFile};
use crate::utils::error::{BokslDirError, Result};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};

/// 마운트 포인트 정보
#[derive(Debug, Clone)]
pub struct MountPoint {
    pub name: String,
    pub path: PathBuf,
}

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

            // 링크 자체 메타데이터
            let Ok(link_metadata) = fs::symlink_metadata(&entry_path) else {
                continue;
            };

            // 파일 이름
            let name = entry.file_name().to_string_lossy().to_string();

            // 파일 타입 판단
            let file_type = self.get_file_type(&entry_path, &link_metadata);

            // 표시용 메타데이터 (symlink는 대상 메타데이터 우선)
            let display_metadata = if file_type == FileType::Symlink {
                fs::metadata(&entry_path).ok().unwrap_or(link_metadata)
            } else {
                link_metadata
            };

            // 크기 (디렉토리/symlink 디렉토리는 0)
            let size = match file_type {
                FileType::Directory => 0,
                FileType::Symlink => {
                    if display_metadata.is_file() {
                        display_metadata.len()
                    } else {
                        0
                    }
                }
                _ => display_metadata.len(),
            };

            // 수정 시간
            let modified = display_metadata
                .modified()
                .unwrap_or_else(|_| std::time::SystemTime::now());
            // 생성 시간 (없으면 수정 시간 fallback)
            let created = display_metadata.created().unwrap_or(modified);

            // 권한 (Unix 계열에서만)
            let permissions = Some(display_metadata.permissions());

            // 숨김 파일 여부
            let is_hidden = self.is_hidden(&entry_path);

            let mut file_entry = FileEntry::new(
                name,
                entry_path,
                file_type,
                size,
                modified,
                created,
                permissions,
                is_hidden,
            );

            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                file_entry.owner = Some(display_metadata.uid().to_string());
                file_entry.group = Some(display_metadata.gid().to_string());
            }

            entries.push(file_entry);
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

    /// OS 기본 프로그램으로 파일 열기
    pub fn open_with_default_app(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(BokslDirError::PathNotFound {
                path: path.to_path_buf(),
            });
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            let status = Command::new("open").arg(path).status().map_err(|e| {
                BokslDirError::ExternalOpenFailed {
                    path: path.to_path_buf(),
                    reason: e.to_string(),
                }
            })?;

            if status.success() {
                Ok(())
            } else {
                Err(BokslDirError::ExternalOpenFailed {
                    path: path.to_path_buf(),
                    reason: format!("open command exited with status {}", status),
                })
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            Err(BokslDirError::ExternalOpenFailed {
                path: path.to_path_buf(),
                reason: "Unsupported platform for Phase 7.1 (macOS only)".to_string(),
            })
        }
    }

    // === Phase 5.3: 마운트 포인트 ===

    /// 시스템 마운트 포인트 목록 반환
    #[allow(clippy::unused_self)]
    pub fn list_mount_points(&self) -> Vec<MountPoint> {
        let mut points = Vec::new();

        // 홈 디렉토리
        if let Ok(home) = std::env::var("HOME") {
            let home_path = PathBuf::from(&home);
            if home_path.is_dir() {
                points.push(MountPoint {
                    name: format!("~ ({})", home),
                    path: home_path,
                });
            }
        }

        // 루트
        let root = PathBuf::from("/");
        if root.is_dir() {
            points.push(MountPoint {
                name: "/".to_string(),
                path: root,
            });
        }

        // macOS: /Volumes/*
        #[cfg(target_os = "macos")]
        {
            let volumes = PathBuf::from("/Volumes");
            if let Ok(entries) = fs::read_dir(&volumes) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        points.push(MountPoint {
                            name: format!("/Volumes/{}", name),
                            path,
                        });
                    }
                }
            }
        }

        // Linux: /mnt/*, /media/$USER/*
        #[cfg(target_os = "linux")]
        {
            for base in &["/mnt", "/media"] {
                let base_path = PathBuf::from(base);
                if let Ok(entries) = fs::read_dir(&base_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let name = path.to_string_lossy().to_string();
                            points.push(MountPoint { name, path });
                        }
                    }
                }
            }
        }

        points
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
            let metadata = fs::symlink_metadata(path).map_err(BokslDirError::Io)?;

            if metadata.file_type().is_symlink() {
                total_bytes += self.symlink_target_file_size(path);
                total_files += 1;
            } else if metadata.is_file() {
                total_bytes += metadata.len();
                total_files += 1;
            } else if metadata.is_dir() {
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
            let metadata = fs::symlink_metadata(&entry_path).map_err(BokslDirError::Io)?;

            if metadata.file_type().is_symlink() {
                total_bytes += self.symlink_target_file_size(&entry_path);
                total_files += 1;
            } else if metadata.is_file() {
                total_bytes += metadata.len();
                total_files += 1;
            } else if metadata.is_dir() {
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

    // === Phase 3.3: 파일 삭제 메서드 ===

    /// 단일 파일 영구 삭제
    ///
    /// 반환값: 삭제된 파일 크기 (바이트)
    #[allow(clippy::unused_self)]
    pub fn delete_file(&self, path: &Path) -> Result<u64> {
        if !path.exists() {
            return Err(BokslDirError::PathNotFound {
                path: path.to_path_buf(),
            });
        }

        let size = path.metadata().map(|m| m.len()).unwrap_or(0);

        fs::remove_file(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                BokslDirError::PermissionDenied {
                    path: path.to_path_buf(),
                }
            } else {
                BokslDirError::DeleteFailed {
                    path: path.to_path_buf(),
                    reason: e.to_string(),
                }
            }
        })?;

        Ok(size)
    }

    /// 디렉토리 재귀 영구 삭제
    ///
    /// 반환값: 삭제된 총 바이트 수
    pub fn delete_directory(&self, path: &Path) -> Result<u64> {
        if !path.exists() {
            return Err(BokslDirError::PathNotFound {
                path: path.to_path_buf(),
            });
        }

        if !path.is_dir() {
            return Err(BokslDirError::NotADirectory {
                path: path.to_path_buf(),
            });
        }

        // 삭제 전 크기 계산
        let (total_bytes, _) = self.calculate_total_size(&[path.to_path_buf()])?;

        fs::remove_dir_all(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                BokslDirError::PermissionDenied {
                    path: path.to_path_buf(),
                }
            } else {
                BokslDirError::DeleteFailed {
                    path: path.to_path_buf(),
                    reason: e.to_string(),
                }
            }
        })?;

        Ok(total_bytes)
    }

    /// 휴지통으로 이동 (trash crate 래퍼)
    #[allow(clippy::unused_self)]
    pub fn trash_items(&self, paths: &[PathBuf]) -> Result<()> {
        trash::delete_all(paths).map_err(|e| BokslDirError::DeleteFailed {
            path: paths.first().cloned().unwrap_or_default(),
            reason: e.to_string(),
        })
    }

    // === Phase 3.4: 디렉토리 생성, 이름 변경 ===

    /// 새 디렉토리 생성
    #[allow(clippy::unused_self)]
    pub fn create_directory(&self, path: &Path) -> Result<()> {
        if path.exists() {
            return Err(BokslDirError::FileExists {
                path: path.to_path_buf(),
            });
        }

        fs::create_dir(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                BokslDirError::PermissionDenied {
                    path: path.to_path_buf(),
                }
            } else {
                BokslDirError::Io(e)
            }
        })
    }

    /// 파일/디렉토리 이름 변경
    #[allow(clippy::unused_self)]
    pub fn rename_path(&self, src: &Path, dest: &Path) -> Result<()> {
        if !src.exists() {
            return Err(BokslDirError::PathNotFound {
                path: src.to_path_buf(),
            });
        }

        if dest.exists() {
            return Err(BokslDirError::FileExists {
                path: dest.to_path_buf(),
            });
        }

        fs::rename(src, dest).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                BokslDirError::PermissionDenied {
                    path: src.to_path_buf(),
                }
            } else {
                BokslDirError::RenameFailed {
                    src: src.to_path_buf(),
                    dest: dest.to_path_buf(),
                    reason: e.to_string(),
                }
            }
        })
    }

    /// 소스 목록을 평탄화하여 개별 파일 목록 생성
    ///
    /// 디렉토리는 재귀적으로 탐색하며 디렉토리 엔트리도 포함합니다.
    pub fn flatten_sources(
        &self,
        sources: &[std::path::PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<FlattenedFile>> {
        let mut result = Vec::new();

        for source in sources {
            let file_name = source.file_name().unwrap_or_default();
            let dest_base = dest_dir.join(file_name);
            let metadata = fs::symlink_metadata(source).map_err(BokslDirError::Io)?;

            if metadata.file_type().is_symlink() {
                let entry_kind = self.classify_symlink_entry_kind(source);
                result.push(FlattenedFile {
                    entry_kind,
                    source: source.clone(),
                    dest: dest_base,
                    size: if entry_kind == FlattenedEntryKind::SymlinkFile {
                        self.symlink_target_file_size(source)
                    } else {
                        0
                    },
                });
            } else if metadata.is_file() {
                result.push(FlattenedFile {
                    entry_kind: FlattenedEntryKind::File,
                    source: source.clone(),
                    dest: dest_base,
                    size: metadata.len(),
                });
            } else if metadata.is_dir() {
                result.push(FlattenedFile {
                    entry_kind: FlattenedEntryKind::Directory,
                    source: source.clone(),
                    dest: dest_base.clone(),
                    size: 0,
                });
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
        result: &mut Vec<FlattenedFile>,
    ) -> Result<()> {
        for entry in fs::read_dir(current_source).map_err(BokslDirError::Io)? {
            let entry = entry.map_err(BokslDirError::Io)?;
            let entry_path = entry.path();
            let metadata = fs::symlink_metadata(&entry_path).map_err(BokslDirError::Io)?;

            // 상대 경로 계산
            let relative = entry_path
                .strip_prefix(base_source)
                .unwrap_or(&entry_path)
                .to_path_buf();
            let dest_path = dest_base.join(&relative);

            if metadata.file_type().is_symlink() {
                let entry_kind = self.classify_symlink_entry_kind(&entry_path);
                result.push(FlattenedFile {
                    entry_kind,
                    source: entry_path.clone(),
                    dest: dest_path,
                    size: if entry_kind == FlattenedEntryKind::SymlinkFile {
                        self.symlink_target_file_size(&entry_path)
                    } else {
                        0
                    },
                });
            } else if metadata.is_file() {
                result.push(FlattenedFile {
                    entry_kind: FlattenedEntryKind::File,
                    source: entry_path,
                    dest: dest_path,
                    size: metadata.len(),
                });
            } else if metadata.is_dir() {
                result.push(FlattenedFile {
                    entry_kind: FlattenedEntryKind::Directory,
                    source: entry_path.clone(),
                    dest: dest_path.clone(),
                    size: 0,
                });
                self.flatten_directory(base_source, &entry_path, dest_base, result)?;
            }
        }

        Ok(())
    }

    fn symlink_target_file_size(&self, path: &Path) -> u64 {
        fs::metadata(path)
            .map(|m| if m.is_file() { m.len() } else { 0 })
            .unwrap_or(0)
    }

    fn classify_symlink_entry_kind(&self, path: &Path) -> FlattenedEntryKind {
        match fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => FlattenedEntryKind::SymlinkDirectory,
            _ => FlattenedEntryKind::SymlinkFile,
        }
    }

    pub fn collect_move_cleanup_dirs(&self, flattened: &[FlattenedFile]) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = flattened
            .iter()
            .filter(|f| f.entry_kind == FlattenedEntryKind::Directory)
            .map(|f| f.source.clone())
            .collect();

        dirs.sort();
        dirs.dedup();
        dirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
        dirs
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
    use crate::models::operation::FlattenedEntryKind;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

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
    fn test_create_directory() {
        let fs_instance = FileSystem::new();
        let temp_dir = std::env::temp_dir().join("boksldir_test_mkdir");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);

        let new_dir = temp_dir.join("new_folder");
        assert!(fs_instance.create_directory(&new_dir).is_ok());
        assert!(new_dir.is_dir());

        // 이미 존재하면 에러
        let result = fs_instance.create_directory(&new_dir);
        assert!(result.is_err());
        match result {
            Err(BokslDirError::FileExists { .. }) => {}
            _ => panic!("Expected FileExists error"),
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_rename_path() {
        let fs_instance = FileSystem::new();
        let temp_dir = std::env::temp_dir().join("boksldir_test_rename");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);

        // 파일 이름 변경
        let src = temp_dir.join("old.txt");
        let dest = temp_dir.join("new.txt");
        let mut file = File::create(&src).unwrap();
        writeln!(file, "test").unwrap();

        assert!(fs_instance.rename_path(&src, &dest).is_ok());
        assert!(!src.exists());
        assert!(dest.exists());

        // 이미 존재하는 대상
        let src2 = temp_dir.join("another.txt");
        let _ = File::create(&src2).unwrap();
        let result = fs_instance.rename_path(&src2, &dest);
        assert!(result.is_err());
        match result {
            Err(BokslDirError::FileExists { .. }) => {}
            _ => panic!("Expected FileExists error"),
        }

        let _ = fs::remove_dir_all(&temp_dir);
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

    #[test]
    fn test_flatten_sources_keeps_empty_directories() {
        let fs = FileSystem::new();
        let temp = TempDir::new().unwrap();
        let source_root = temp.path().join("src");
        let empty_dir = source_root.join("empty");
        let nested_dir = source_root.join("nested");
        let nested_file = nested_dir.join("file.txt");
        let dest_root = temp.path().join("dest");

        fs::create_dir_all(&empty_dir).unwrap();
        fs::create_dir_all(&nested_dir).unwrap();
        fs::write(&nested_file, "hello").unwrap();
        fs::create_dir_all(&dest_root).unwrap();

        let flattened = fs
            .flatten_sources(std::slice::from_ref(&source_root), &dest_root)
            .unwrap();

        let src_name = source_root.file_name().unwrap();
        let expected_src_dir = dest_root.join(src_name);
        let expected_empty_dir = expected_src_dir.join("empty");
        let expected_file = expected_src_dir.join("nested").join("file.txt");

        assert!(flattened.iter().any(|f| {
            f.entry_kind == FlattenedEntryKind::Directory && f.dest == expected_src_dir
        }));
        assert!(flattened.iter().any(|f| {
            f.entry_kind == FlattenedEntryKind::Directory && f.dest == expected_empty_dir
        }));
        assert!(flattened
            .iter()
            .any(|f| f.entry_kind == FlattenedEntryKind::File && f.dest == expected_file));
    }

    #[cfg(unix)]
    #[test]
    fn test_read_directory_detects_symlink_type() {
        let fs = FileSystem::new();
        let temp = TempDir::new().unwrap();
        let dir = temp.path();
        let target = dir.join("target.txt");
        let link = dir.join("target_link");

        fs::write(&target, "link target").unwrap();
        unix_fs::symlink(&target, &link).unwrap();

        let entries = fs.read_directory(dir).unwrap();
        let symlink_entry = entries
            .iter()
            .find(|entry| entry.name == "target_link")
            .expect("symlink entry not found");
        assert_eq!(symlink_entry.file_type, FileType::Symlink);
    }

    #[cfg(unix)]
    #[test]
    fn test_flatten_sources_classifies_symlink_directory() {
        let fs = FileSystem::new();
        let temp = TempDir::new().unwrap();
        let source_root = temp.path().join("src");
        let target_dir = temp.path().join("target_dir");
        let link_to_dir = source_root.join("dir_link");
        let dest_root = temp.path().join("dest");

        fs::create_dir_all(&source_root).unwrap();
        fs::create_dir_all(&target_dir).unwrap();
        unix_fs::symlink(&target_dir, &link_to_dir).unwrap();
        fs::create_dir_all(&dest_root).unwrap();

        let flattened = fs
            .flatten_sources(std::slice::from_ref(&source_root), &dest_root)
            .unwrap();

        let entry = flattened
            .iter()
            .find(|f| f.source == link_to_dir)
            .expect("symlink directory should be flattened");
        assert_eq!(entry.entry_kind, FlattenedEntryKind::SymlinkDirectory);
        assert_eq!(entry.size, 0);
    }

    #[cfg(unix)]
    #[test]
    fn test_flatten_sources_does_not_recurse_into_symlink_dirs() {
        let fs = FileSystem::new();
        let temp = TempDir::new().unwrap();
        let source_root = temp.path().join("src");
        let local_file = source_root.join("local.txt");
        let external_dir = temp.path().join("external");
        let external_file = external_dir.join("outside.txt");
        let link_to_external = source_root.join("external_link");
        let dest_root = temp.path().join("dest");

        fs::create_dir_all(&source_root).unwrap();
        fs::write(&local_file, "local").unwrap();
        fs::create_dir_all(&external_dir).unwrap();
        fs::write(&external_file, "outside").unwrap();
        unix_fs::symlink(&external_dir, &link_to_external).unwrap();
        fs::create_dir_all(&dest_root).unwrap();

        let flattened = fs
            .flatten_sources(std::slice::from_ref(&source_root), &dest_root)
            .unwrap();

        let src_name = source_root.file_name().unwrap();
        let base_dest = dest_root.join(src_name);
        let expected_link_dest = base_dest.join("external_link");
        let outside_dest = base_dest.join("external_link").join("outside.txt");

        assert!(flattened.iter().any(|f| {
            f.entry_kind == FlattenedEntryKind::SymlinkDirectory
                && f.source == link_to_external
                && f.dest == expected_link_dest
        }));
        assert!(!flattened.iter().any(|f| f.dest == outside_dest));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_open_with_default_app_nonexistent_path_returns_path_not_found() {
        let fs = FileSystem::new();
        let missing = PathBuf::from("/tmp/boksldir-open-missing-1234567890.txt");

        let result = fs.open_with_default_app(&missing);
        match result {
            Err(BokslDirError::PathNotFound { path }) => assert_eq!(path, missing),
            other => panic!("expected PathNotFound, got {:?}", other),
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_open_with_default_app_reports_platform_not_supported() {
        let fs = FileSystem::new();
        let current = std::env::current_dir().unwrap();

        let result = fs.open_with_default_app(&current);
        match result {
            Err(BokslDirError::ExternalOpenFailed { reason, .. }) => {
                assert!(reason.contains("macOS only"));
            }
            other => panic!("expected ExternalOpenFailed, got {:?}", other),
        }
    }
}
