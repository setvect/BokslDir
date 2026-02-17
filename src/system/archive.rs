#![allow(dead_code)]

use crate::utils::error::{BokslDirError, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sevenz_rust2::Error as SevenZError;
use sevenz_rust2::Password as SevenZPassword;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tar::{Archive as TarArchive, Builder as TarBuilder};
use zip::result::ZipError;
use zip::write::SimpleFileOptions as ZipFileOptions;
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter};
use zstd::stream::read::Decoder as ZstdDecoder;
use zstd::stream::write::Encoder as ZstdEncoder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    Tar,
    TarGz,
    TarZst,
    SevenZ,
    Jar,
    War,
}

impl ArchiveFormat {
    pub fn display_name(&self) -> &'static str {
        match self {
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::Tar => "tar",
            ArchiveFormat::TarGz => "tar.gz",
            ArchiveFormat::TarZst => "tar.zst",
            ArchiveFormat::SevenZ => "7z",
            ArchiveFormat::Jar => "jar",
            ArchiveFormat::War => "war",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct ArchiveCreateRequest {
    pub sources: Vec<PathBuf>,
    pub output_path: PathBuf,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ArchiveExtractRequest {
    pub archive_path: PathBuf,
    pub dest_dir: PathBuf,
    pub password: Option<String>,
    pub overwrite_existing: bool,
    pub overwrite_entries: Vec<String>,
    pub skip_existing_entries: Vec<String>,
    pub skip_all_existing: bool,
}

#[derive(Debug, Clone)]
pub struct ArchiveSummary {
    pub total_files: usize,
    pub total_bytes: u64,
    pub items_processed: usize,
    pub items_failed: usize,
    pub errors: Vec<String>,
    pub cancelled: bool,
}

impl ArchiveSummary {
    fn new(total_files: usize, total_bytes: u64) -> Self {
        Self {
            total_files,
            total_bytes,
            items_processed: 0,
            items_failed: 0,
            errors: Vec::new(),
            cancelled: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArchiveProgressEvent {
    pub current_file: String,
    pub files_completed: usize,
    pub total_files: usize,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub items_processed: usize,
    pub items_failed: usize,
}

#[derive(Debug, Clone)]
struct ArchiveSourceItem {
    source_path: PathBuf,
    archive_path: PathBuf,
    is_dir: bool,
    size: u64,
}

pub fn detect_archive_format(path: &Path) -> Option<ArchiveFormat> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Some(ArchiveFormat::TarGz);
    }
    if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        return Some(ArchiveFormat::TarZst);
    }
    match path
        .extension()
        .and_then(OsStr::to_str)?
        .to_lowercase()
        .as_str()
    {
        "zip" => Some(ArchiveFormat::Zip),
        "tar" => Some(ArchiveFormat::Tar),
        "7z" => Some(ArchiveFormat::SevenZ),
        "jar" => Some(ArchiveFormat::Jar),
        "war" => Some(ArchiveFormat::War),
        _ => None,
    }
}

pub fn supports_password(format: ArchiveFormat) -> bool {
    matches!(format, ArchiveFormat::Zip | ArchiveFormat::SevenZ)
}

pub fn list_entries(path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>> {
    let format =
        detect_archive_format(path).ok_or_else(|| BokslDirError::ArchiveUnsupportedFormat {
            path: path.to_path_buf(),
        })?;

    match format {
        ArchiveFormat::Zip | ArchiveFormat::Jar | ArchiveFormat::War => {
            list_zip_entries(path, password)
        }
        ArchiveFormat::Tar => list_tar_entries(path),
        ArchiveFormat::TarGz => list_tar_gz_entries(path),
        ArchiveFormat::TarZst => list_tar_zst_entries(path),
        ArchiveFormat::SevenZ => list_7z_entries(path, password),
    }
}

pub fn create_archive(
    request: &ArchiveCreateRequest,
    progress_tx: Sender<ArchiveProgressEvent>,
    cancel_flag: Arc<AtomicBool>,
) -> Result<ArchiveSummary> {
    let format = detect_archive_format(&request.output_path).ok_or_else(|| {
        BokslDirError::ArchiveUnsupportedFormat {
            path: request.output_path.clone(),
        }
    })?;

    if request.sources.is_empty() {
        return Err(BokslDirError::ArchiveCreateFailed {
            path: request.output_path.clone(),
            reason: "No source selected".to_string(),
        });
    }

    if request.output_path.exists() {
        return Err(BokslDirError::ArchiveCreateFailed {
            path: request.output_path.clone(),
            reason: "Destination archive already exists".to_string(),
        });
    }

    let items = collect_source_items(&request.sources)?;
    let total_files = items.len();
    let total_bytes = items.iter().map(|i| i.size).sum::<u64>();
    let mut summary = ArchiveSummary::new(total_files, total_bytes);
    let mut files_completed = 0usize;
    let mut bytes_processed = 0u64;

    let _ = progress_tx.send(ArchiveProgressEvent {
        current_file: String::new(),
        files_completed,
        total_files,
        bytes_processed,
        total_bytes,
        items_processed: summary.items_processed,
        items_failed: summary.items_failed,
    });

    match format {
        ArchiveFormat::Zip | ArchiveFormat::Jar | ArchiveFormat::War => {
            create_zip_archive(
                &request.output_path,
                &items,
                request.password.as_deref(),
                &progress_tx,
                &cancel_flag,
                &mut summary,
                &mut files_completed,
                &mut bytes_processed,
            )?;
        }
        ArchiveFormat::Tar => {
            create_tar_archive(
                &request.output_path,
                &items,
                &progress_tx,
                &cancel_flag,
                &mut summary,
                &mut files_completed,
                &mut bytes_processed,
            )?;
        }
        ArchiveFormat::TarGz => {
            create_tar_gz_archive(
                &request.output_path,
                &items,
                &progress_tx,
                &cancel_flag,
                &mut summary,
                &mut files_completed,
                &mut bytes_processed,
            )?;
        }
        ArchiveFormat::TarZst => {
            create_tar_zst_archive(
                &request.output_path,
                &items,
                &progress_tx,
                &cancel_flag,
                &mut summary,
                &mut files_completed,
                &mut bytes_processed,
            )?;
        }
        ArchiveFormat::SevenZ => {
            create_7z_archive(
                &request.output_path,
                &request.sources,
                request.password.as_deref(),
                &progress_tx,
                &cancel_flag,
                &mut summary,
                total_files,
                total_bytes,
            )?;
        }
    }

    Ok(summary)
}

pub fn extract_archive(
    request: &ArchiveExtractRequest,
    progress_tx: Sender<ArchiveProgressEvent>,
    cancel_flag: Arc<AtomicBool>,
) -> Result<ArchiveSummary> {
    let format = detect_archive_format(&request.archive_path).ok_or_else(|| {
        BokslDirError::ArchiveUnsupportedFormat {
            path: request.archive_path.clone(),
        }
    })?;

    if !request.dest_dir.exists() || !request.dest_dir.is_dir() {
        return Err(BokslDirError::ArchiveExtractFailed {
            path: request.archive_path.clone(),
            reason: format!(
                "Destination directory does not exist: {}",
                request.dest_dir.display()
            ),
        });
    }

    let list = list_entries(&request.archive_path, request.password.as_deref())?;
    let total_files = list.len();
    let total_bytes = list.iter().map(|e| e.size).sum::<u64>();
    let mut summary = ArchiveSummary::new(total_files, total_bytes);

    let _ = progress_tx.send(ArchiveProgressEvent {
        current_file: String::new(),
        files_completed: 0,
        total_files,
        bytes_processed: 0,
        total_bytes,
        items_processed: 0,
        items_failed: 0,
    });

    match format {
        ArchiveFormat::Zip | ArchiveFormat::Jar | ArchiveFormat::War => extract_zip_archive(
            request,
            &progress_tx,
            &cancel_flag,
            &mut summary,
            total_files,
            total_bytes,
        )?,
        ArchiveFormat::Tar => extract_tar_archive(
            request,
            &progress_tx,
            &cancel_flag,
            &mut summary,
            total_files,
            total_bytes,
        )?,
        ArchiveFormat::TarGz => extract_tar_gz_archive(
            request,
            &progress_tx,
            &cancel_flag,
            &mut summary,
            total_files,
            total_bytes,
        )?,
        ArchiveFormat::TarZst => extract_tar_zst_archive(
            request,
            &progress_tx,
            &cancel_flag,
            &mut summary,
            total_files,
            total_bytes,
        )?,
        ArchiveFormat::SevenZ => extract_7z_archive(
            request,
            &progress_tx,
            &cancel_flag,
            &mut summary,
            total_files,
            total_bytes,
        )?,
    }

    Ok(summary)
}

pub fn list_extract_conflicts(
    archive_path: &Path,
    dest_dir: &Path,
    password: Option<&str>,
) -> Result<Vec<String>> {
    let entries = list_entries(archive_path, password)?;
    let mut conflicts = BTreeSet::new();

    for entry in entries {
        let raw_path = PathBuf::from(&entry.path);
        let Some(dest_path) = sanitize_extract_path(dest_dir, &raw_path) else {
            continue;
        };
        if dest_path.exists() && !(entry.is_dir && dest_path.is_dir()) {
            conflicts.insert(entry.path);
        }
    }

    Ok(conflicts.into_iter().collect())
}

fn collect_source_items(sources: &[PathBuf]) -> Result<Vec<ArchiveSourceItem>> {
    let mut items = Vec::new();
    for source in sources {
        let name = source
            .file_name()
            .ok_or_else(|| BokslDirError::ArchiveCreateFailed {
                path: source.clone(),
                reason: "Invalid source name".to_string(),
            })?;
        let archive_path = PathBuf::from(name);
        collect_source_item_recursive(source, &archive_path, &mut items)?;
    }
    Ok(items)
}

fn collect_source_item_recursive(
    source_path: &Path,
    archive_path: &Path,
    out: &mut Vec<ArchiveSourceItem>,
) -> Result<()> {
    let meta = fs::symlink_metadata(source_path).map_err(BokslDirError::Io)?;
    if meta.is_dir() {
        out.push(ArchiveSourceItem {
            source_path: source_path.to_path_buf(),
            archive_path: archive_path.to_path_buf(),
            is_dir: true,
            size: 0,
        });
        for entry in fs::read_dir(source_path).map_err(BokslDirError::Io)? {
            let entry = entry.map_err(BokslDirError::Io)?;
            let child_source = entry.path();
            let child_archive = archive_path.join(entry.file_name());
            collect_source_item_recursive(&child_source, &child_archive, out)?;
        }
    } else {
        out.push(ArchiveSourceItem {
            source_path: source_path.to_path_buf(),
            archive_path: archive_path.to_path_buf(),
            is_dir: false,
            size: meta.len(),
        });
    }
    Ok(())
}

fn archive_display_path(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(v) => Some(v.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn should_cancel(cancel_flag: &Arc<AtomicBool>) -> bool {
    cancel_flag.load(Ordering::Relaxed)
}

fn normalize_entry_name(name: &str) -> String {
    name.replace('\\', "/").trim_matches('/').to_string()
}

fn matches_entry(list: &[String], entry_name: &str) -> bool {
    let normalized = normalize_entry_name(entry_name);
    list.iter()
        .any(|item| normalize_entry_name(item) == normalized)
}

fn should_overwrite_existing(request: &ArchiveExtractRequest, entry_name: &str) -> bool {
    request.overwrite_existing || matches_entry(&request.overwrite_entries, entry_name)
}

fn should_skip_existing(request: &ArchiveExtractRequest, entry_name: &str) -> bool {
    request.skip_all_existing || matches_entry(&request.skip_existing_entries, entry_name)
}

fn send_progress(
    progress_tx: &Sender<ArchiveProgressEvent>,
    current_file: String,
    files_completed: usize,
    total_files: usize,
    bytes_processed: u64,
    total_bytes: u64,
    summary: &ArchiveSummary,
) {
    let _ = progress_tx.send(ArchiveProgressEvent {
        current_file,
        files_completed,
        total_files,
        bytes_processed,
        total_bytes,
        items_processed: summary.items_processed,
        items_failed: summary.items_failed,
    });
}

fn map_zip_list_error(path: &Path, error: ZipError, password: Option<&str>) -> BokslDirError {
    match error {
        ZipError::UnsupportedArchive(detail)
            if detail == ZipError::PASSWORD_REQUIRED && password.is_none() =>
        {
            BokslDirError::ArchivePasswordRequired {
                path: path.to_path_buf(),
            }
        }
        ZipError::InvalidPassword => BokslDirError::ArchiveInvalidPassword {
            path: path.to_path_buf(),
            reason: "Invalid ZIP password".to_string(),
        },
        ZipError::UnsupportedArchive(detail)
            if password.is_some() && detail.to_ascii_lowercase().contains("password") =>
        {
            BokslDirError::ArchiveInvalidPassword {
                path: path.to_path_buf(),
                reason: detail.to_string(),
            }
        }
        other => BokslDirError::ArchiveListFailed {
            path: path.to_path_buf(),
            reason: other.to_string(),
        },
    }
}

fn map_zip_extract_error(path: &Path, error: ZipError, password: Option<&str>) -> BokslDirError {
    match error {
        ZipError::UnsupportedArchive(detail)
            if detail == ZipError::PASSWORD_REQUIRED && password.is_none() =>
        {
            BokslDirError::ArchivePasswordRequired {
                path: path.to_path_buf(),
            }
        }
        ZipError::InvalidPassword => BokslDirError::ArchiveInvalidPassword {
            path: path.to_path_buf(),
            reason: "Invalid ZIP password".to_string(),
        },
        ZipError::UnsupportedArchive(detail)
            if password.is_some() && detail.to_ascii_lowercase().contains("password") =>
        {
            BokslDirError::ArchiveInvalidPassword {
                path: path.to_path_buf(),
                reason: detail.to_string(),
            }
        }
        other => BokslDirError::ArchiveExtractFailed {
            path: path.to_path_buf(),
            reason: other.to_string(),
        },
    }
}

fn map_7z_list_error(path: &Path, error: SevenZError, password: Option<&str>) -> BokslDirError {
    match error {
        SevenZError::PasswordRequired if password.is_none() => {
            BokslDirError::ArchivePasswordRequired {
                path: path.to_path_buf(),
            }
        }
        SevenZError::PasswordRequired => BokslDirError::ArchiveInvalidPassword {
            path: path.to_path_buf(),
            reason: "Password required".to_string(),
        },
        SevenZError::MaybeBadPassword(inner) => BokslDirError::ArchiveInvalidPassword {
            path: path.to_path_buf(),
            reason: inner.to_string(),
        },
        other => {
            let reason = other.to_string();
            if password.is_some() && reason.to_ascii_lowercase().contains("password") {
                BokslDirError::ArchiveInvalidPassword {
                    path: path.to_path_buf(),
                    reason,
                }
            } else {
                BokslDirError::ArchiveListFailed {
                    path: path.to_path_buf(),
                    reason,
                }
            }
        }
    }
}

fn map_7z_extract_error(path: &Path, error: SevenZError, password: Option<&str>) -> BokslDirError {
    match error {
        SevenZError::PasswordRequired if password.is_none() => {
            BokslDirError::ArchivePasswordRequired {
                path: path.to_path_buf(),
            }
        }
        SevenZError::PasswordRequired => BokslDirError::ArchiveInvalidPassword {
            path: path.to_path_buf(),
            reason: "Password required".to_string(),
        },
        SevenZError::MaybeBadPassword(inner) => BokslDirError::ArchiveInvalidPassword {
            path: path.to_path_buf(),
            reason: inner.to_string(),
        },
        other => {
            let reason = other.to_string();
            if password.is_some() && reason.to_ascii_lowercase().contains("password") {
                BokslDirError::ArchiveInvalidPassword {
                    path: path.to_path_buf(),
                    reason,
                }
            } else {
                BokslDirError::ArchiveExtractFailed {
                    path: path.to_path_buf(),
                    reason,
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn create_zip_archive(
    output_path: &Path,
    items: &[ArchiveSourceItem],
    password: Option<&str>,
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    files_completed: &mut usize,
    bytes_processed: &mut u64,
) -> Result<()> {
    let file = File::create(output_path).map_err(BokslDirError::Io)?;
    let mut writer = ZipWriter::new(file);

    let mut options = ZipFileOptions::default().compression_method(CompressionMethod::Deflated);
    if let Some(pass) = password {
        options = options.with_aes_encryption(AesMode::Aes256, pass);
    }

    for item in items {
        if should_cancel(cancel_flag) {
            summary.cancelled = true;
            return Ok(());
        }

        let mut name = archive_display_path(&item.archive_path);
        if item.is_dir && !name.ends_with('/') {
            name.push('/');
        }

        let result = if item.is_dir {
            writer
                .add_directory(name.clone(), options)
                .map(|_| 0u64)
                .map_err(|e| e.to_string())
        } else {
            (|| -> std::result::Result<u64, String> {
                writer
                    .start_file(name.clone(), options)
                    .map_err(|e| e.to_string())?;
                let mut src = File::open(&item.source_path).map_err(|e| e.to_string())?;
                let copied = io::copy(&mut src, &mut writer).map_err(|e| e.to_string())?;
                Ok(copied)
            })()
        };

        match result {
            Ok(copied) => {
                *files_completed += 1;
                *bytes_processed = bytes_processed.saturating_add(copied);
                summary.items_processed += 1;
                send_progress(
                    progress_tx,
                    name,
                    *files_completed,
                    summary.total_files,
                    *bytes_processed,
                    summary.total_bytes,
                    summary,
                );
            }
            Err(e) => {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary.errors.push(format!("{}: {}", name, e));
            }
        }
    }

    writer
        .finish()
        .map_err(|e| BokslDirError::ArchiveCreateFailed {
            path: output_path.to_path_buf(),
            reason: e.to_string(),
        })?;

    Ok(())
}

fn create_tar_archive(
    output_path: &Path,
    items: &[ArchiveSourceItem],
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    files_completed: &mut usize,
    bytes_processed: &mut u64,
) -> Result<()> {
    let file = File::create(output_path).map_err(BokslDirError::Io)?;
    let mut builder = TarBuilder::new(file);
    create_tar_like_archive(
        &mut builder,
        items,
        progress_tx,
        cancel_flag,
        summary,
        files_completed,
        bytes_processed,
    )?;
    builder.finish().map_err(BokslDirError::Io)?;
    Ok(())
}

fn create_tar_gz_archive(
    output_path: &Path,
    items: &[ArchiveSourceItem],
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    files_completed: &mut usize,
    bytes_processed: &mut u64,
) -> Result<()> {
    let file = File::create(output_path).map_err(BokslDirError::Io)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = TarBuilder::new(encoder);
    create_tar_like_archive(
        &mut builder,
        items,
        progress_tx,
        cancel_flag,
        summary,
        files_completed,
        bytes_processed,
    )?;
    let encoder = builder.into_inner().map_err(BokslDirError::Io)?;
    encoder.finish().map_err(BokslDirError::Io)?;
    Ok(())
}

fn create_tar_zst_archive(
    output_path: &Path,
    items: &[ArchiveSourceItem],
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    files_completed: &mut usize,
    bytes_processed: &mut u64,
) -> Result<()> {
    let file = File::create(output_path).map_err(BokslDirError::Io)?;
    let encoder = ZstdEncoder::new(file, 3).map_err(BokslDirError::Io)?;
    let mut builder = TarBuilder::new(encoder);
    create_tar_like_archive(
        &mut builder,
        items,
        progress_tx,
        cancel_flag,
        summary,
        files_completed,
        bytes_processed,
    )?;
    let encoder = builder.into_inner().map_err(BokslDirError::Io)?;
    encoder.finish().map_err(BokslDirError::Io)?;
    Ok(())
}

fn create_tar_like_archive<W: Write>(
    builder: &mut TarBuilder<W>,
    items: &[ArchiveSourceItem],
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    files_completed: &mut usize,
    bytes_processed: &mut u64,
) -> Result<()> {
    for item in items {
        if should_cancel(cancel_flag) {
            summary.cancelled = true;
            return Ok(());
        }

        let name = archive_display_path(&item.archive_path);
        let result = if item.is_dir {
            builder
                .append_dir(name.clone(), &item.source_path)
                .map(|_| 0u64)
                .map_err(|e| e.to_string())
        } else {
            (|| -> std::result::Result<u64, String> {
                let mut src = File::open(&item.source_path).map_err(|e| e.to_string())?;
                builder
                    .append_file(name.clone(), &mut src)
                    .map_err(|e| e.to_string())?;
                Ok(item.size)
            })()
        };

        match result {
            Ok(copied) => {
                *files_completed += 1;
                *bytes_processed = bytes_processed.saturating_add(copied);
                summary.items_processed += 1;
                send_progress(
                    progress_tx,
                    name,
                    *files_completed,
                    summary.total_files,
                    *bytes_processed,
                    summary.total_bytes,
                    summary,
                );
            }
            Err(e) => {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary.errors.push(format!("{}: {}", name, e));
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_7z_archive(
    output_path: &Path,
    sources: &[PathBuf],
    password: Option<&str>,
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    total_files: usize,
    total_bytes: u64,
) -> Result<()> {
    if should_cancel(cancel_flag) {
        summary.cancelled = true;
        return Ok(());
    }

    let staging = build_7z_staging_dir(sources)?;
    let result = if let Some(pass) = password {
        sevenz_rust2::compress_to_path_encrypted(&staging, output_path, SevenZPassword::from(pass))
            .map_err(|e| e.to_string())
    } else {
        sevenz_rust2::compress_to_path(&staging, output_path).map_err(|e| e.to_string())
    };
    let _ = fs::remove_dir_all(&staging);

    match result {
        Ok(()) => {
            summary.items_processed = total_files;
            send_progress(
                progress_tx,
                output_path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .unwrap_or("archive")
                    .to_string(),
                total_files,
                total_files,
                total_bytes,
                total_bytes,
                summary,
            );
            Ok(())
        }
        Err(reason) => Err(BokslDirError::ArchiveCreateFailed {
            path: output_path.to_path_buf(),
            reason,
        }),
    }
}

fn build_7z_staging_dir(sources: &[PathBuf]) -> Result<PathBuf> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let staging = std::env::temp_dir().join(format!("boksldir-7z-{}-{}", std::process::id(), now));
    fs::create_dir_all(&staging).map_err(BokslDirError::Io)?;
    for source in sources {
        let Some(name) = source.file_name() else {
            continue;
        };
        let dest = staging.join(name);
        copy_path_recursive(source, &dest)?;
    }
    Ok(staging)
}

fn copy_path_recursive(src: &Path, dest: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(src).map_err(BokslDirError::Io)?;
    if meta.is_dir() {
        fs::create_dir_all(dest).map_err(BokslDirError::Io)?;
        for entry in fs::read_dir(src).map_err(BokslDirError::Io)? {
            let entry = entry.map_err(BokslDirError::Io)?;
            copy_path_recursive(&entry.path(), &dest.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(BokslDirError::Io)?;
    }
    fs::copy(src, dest).map_err(BokslDirError::Io)?;
    Ok(())
}

fn extract_zip_archive(
    request: &ArchiveExtractRequest,
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    total_files: usize,
    total_bytes: u64,
) -> Result<()> {
    let file = File::open(&request.archive_path).map_err(BokslDirError::Io)?;
    let mut archive = ZipArchive::new(file).map_err(|e| BokslDirError::ArchiveExtractFailed {
        path: request.archive_path.clone(),
        reason: e.to_string(),
    })?;

    let mut files_completed = 0usize;
    let mut bytes_processed = 0u64;
    for idx in 0..archive.len() {
        if should_cancel(cancel_flag) {
            summary.cancelled = true;
            return Ok(());
        }

        let password = request.password.as_deref();
        let mut entry = match password {
            Some(pass) => archive
                .by_index_decrypt(idx, pass.as_bytes())
                .map_err(|e| map_zip_extract_error(&request.archive_path, e, password))?,
            None => archive
                .by_index(idx)
                .map_err(|e| map_zip_extract_error(&request.archive_path, e, password))?,
        };

        let raw_path = PathBuf::from(entry.name());
        let name = entry.name().to_string();
        let Some(dest_path) = sanitize_extract_path(&request.dest_dir, &raw_path) else {
            summary.items_processed += 1;
            summary.items_failed += 1;
            summary
                .errors
                .push(format!("{}: blocked unsafe path", entry.name()));
            continue;
        };

        if dest_path.exists() {
            if entry.is_dir() && dest_path.is_dir() {
                files_completed += 1;
                summary.items_processed += 1;
                send_progress(
                    progress_tx,
                    name,
                    files_completed,
                    total_files,
                    bytes_processed,
                    total_bytes,
                    summary,
                );
                continue;
            }
            if should_overwrite_existing(request, &name) {
                if dest_path.is_dir() {
                    if let Err(e) = fs::remove_dir_all(&dest_path) {
                        summary.items_processed += 1;
                        summary.items_failed += 1;
                        summary.errors.push(format!("{}: {}", name, e));
                        continue;
                    }
                } else if let Err(e) = fs::remove_file(&dest_path) {
                    summary.items_processed += 1;
                    summary.items_failed += 1;
                    summary.errors.push(format!("{}: {}", name, e));
                    continue;
                }
            } else if should_skip_existing(request, &name) {
                summary.items_processed += 1;
                send_progress(
                    progress_tx,
                    name,
                    files_completed,
                    total_files,
                    bytes_processed,
                    total_bytes,
                    summary,
                );
                continue;
            } else {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary.errors.push(format!("{}: destination exists", name));
                continue;
            }
        }

        if entry.is_dir() {
            if let Err(e) = fs::create_dir_all(&dest_path) {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary.errors.push(format!("{}: {}", name, e));
                continue;
            }
        } else {
            if let Some(parent) = dest_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match File::create(&dest_path) {
                Ok(mut out) => {
                    if let Err(e) = io::copy(&mut entry, &mut out) {
                        summary.items_processed += 1;
                        summary.items_failed += 1;
                        summary.errors.push(format!("{}: {}", name, e));
                        continue;
                    }
                }
                Err(e) => {
                    summary.items_processed += 1;
                    summary.items_failed += 1;
                    summary.errors.push(format!("{}: {}", name, e));
                    continue;
                }
            }
        }

        files_completed += 1;
        bytes_processed = bytes_processed.saturating_add(entry.size());
        summary.items_processed += 1;
        send_progress(
            progress_tx,
            name,
            files_completed,
            total_files,
            bytes_processed,
            total_bytes,
            summary,
        );
    }
    Ok(())
}

fn extract_tar_archive(
    request: &ArchiveExtractRequest,
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    total_files: usize,
    total_bytes: u64,
) -> Result<()> {
    let file = File::open(&request.archive_path).map_err(BokslDirError::Io)?;
    let archive = TarArchive::new(file);
    extract_tar_like_archive(
        archive,
        request,
        progress_tx,
        cancel_flag,
        summary,
        total_files,
        total_bytes,
    )
}

fn extract_tar_gz_archive(
    request: &ArchiveExtractRequest,
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    total_files: usize,
    total_bytes: u64,
) -> Result<()> {
    let file = File::open(&request.archive_path).map_err(BokslDirError::Io)?;
    let decoder = GzDecoder::new(file);
    let archive = TarArchive::new(decoder);
    extract_tar_like_archive(
        archive,
        request,
        progress_tx,
        cancel_flag,
        summary,
        total_files,
        total_bytes,
    )
}

fn extract_tar_zst_archive(
    request: &ArchiveExtractRequest,
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    total_files: usize,
    total_bytes: u64,
) -> Result<()> {
    let file = File::open(&request.archive_path).map_err(BokslDirError::Io)?;
    let decoder = ZstdDecoder::new(file).map_err(BokslDirError::Io)?;
    let archive = TarArchive::new(decoder);
    extract_tar_like_archive(
        archive,
        request,
        progress_tx,
        cancel_flag,
        summary,
        total_files,
        total_bytes,
    )
}

fn extract_tar_like_archive<R: Read>(
    mut archive: TarArchive<R>,
    request: &ArchiveExtractRequest,
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    total_files: usize,
    total_bytes: u64,
) -> Result<()> {
    let mut files_completed = 0usize;
    let mut bytes_processed = 0u64;
    for entry_result in archive.entries().map_err(BokslDirError::Io)? {
        if should_cancel(cancel_flag) {
            summary.cancelled = true;
            return Ok(());
        }

        let mut entry = match entry_result {
            Ok(v) => v,
            Err(e) => {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary.errors.push(e.to_string());
                continue;
            }
        };

        let path_buf = match entry.path() {
            Ok(v) => v.into_owned(),
            Err(e) => {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary.errors.push(e.to_string());
                continue;
            }
        };
        let name = archive_display_path(&path_buf);
        let is_dir = entry.header().entry_type().is_dir();
        let Some(dest_path) = sanitize_extract_path(&request.dest_dir, &path_buf) else {
            summary.items_processed += 1;
            summary.items_failed += 1;
            summary
                .errors
                .push(format!("{}: blocked unsafe path", name));
            continue;
        };

        if dest_path.exists() {
            if is_dir && dest_path.is_dir() {
                files_completed += 1;
                summary.items_processed += 1;
                send_progress(
                    progress_tx,
                    name,
                    files_completed,
                    total_files,
                    bytes_processed,
                    total_bytes,
                    summary,
                );
                continue;
            }
            if should_overwrite_existing(request, &name) {
                if dest_path.is_dir() {
                    if let Err(e) = fs::remove_dir_all(&dest_path) {
                        summary.items_processed += 1;
                        summary.items_failed += 1;
                        summary.errors.push(format!("{}: {}", name, e));
                        continue;
                    }
                } else if let Err(e) = fs::remove_file(&dest_path) {
                    summary.items_processed += 1;
                    summary.items_failed += 1;
                    summary.errors.push(format!("{}: {}", name, e));
                    continue;
                }
            } else if should_skip_existing(request, &name) {
                summary.items_processed += 1;
                send_progress(
                    progress_tx,
                    name,
                    files_completed,
                    total_files,
                    bytes_processed,
                    total_bytes,
                    summary,
                );
                continue;
            } else {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary.errors.push(format!("{}: destination exists", name));
                continue;
            }
        }

        if let Some(parent) = dest_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let size = entry.size();
        match entry.unpack(&dest_path) {
            Ok(_) => {
                files_completed += 1;
                bytes_processed = bytes_processed.saturating_add(size);
                summary.items_processed += 1;
                send_progress(
                    progress_tx,
                    name,
                    files_completed,
                    total_files,
                    bytes_processed,
                    total_bytes,
                    summary,
                );
            }
            Err(e) => {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary.errors.push(format!("{}: {}", name, e));
            }
        }
    }
    Ok(())
}

fn extract_7z_archive(
    request: &ArchiveExtractRequest,
    progress_tx: &Sender<ArchiveProgressEvent>,
    cancel_flag: &Arc<AtomicBool>,
    summary: &mut ArchiveSummary,
    total_files: usize,
    total_bytes: u64,
) -> Result<()> {
    if should_cancel(cancel_flag) {
        summary.cancelled = true;
        return Ok(());
    }

    let file = File::open(&request.archive_path).map_err(BokslDirError::Io)?;
    let mut files_completed = 0usize;
    let mut bytes_processed = 0u64;
    let dest_root = request.dest_dir.clone();

    let mut extract_fn = |entry: &sevenz_rust2::SevenZArchiveEntry,
                          reader: &mut dyn Read,
                          _output_path: &PathBuf|
     -> std::result::Result<bool, sevenz_rust2::Error> {
        if should_cancel(cancel_flag) {
            summary.cancelled = true;
            return Ok(false);
        }

        let entry_name = entry.name.clone();
        let Some(safe_dest) = sanitize_extract_path(&dest_root, Path::new(&entry_name)) else {
            summary.items_processed += 1;
            summary.items_failed += 1;
            summary
                .errors
                .push(format!("{}: blocked unsafe path", entry_name));
            return Ok(true);
        };

        if safe_dest.exists() {
            if entry.is_directory && safe_dest.is_dir() {
                files_completed += 1;
                summary.items_processed += 1;
                send_progress(
                    progress_tx,
                    entry_name,
                    files_completed,
                    total_files,
                    bytes_processed,
                    total_bytes,
                    summary,
                );
                return Ok(true);
            }
            if should_overwrite_existing(request, &entry_name) {
                if safe_dest.is_dir() {
                    if let Err(e) = fs::remove_dir_all(&safe_dest) {
                        summary.items_processed += 1;
                        summary.items_failed += 1;
                        summary.errors.push(format!("{}: {}", entry_name, e));
                        return Ok(true);
                    }
                } else if let Err(e) = fs::remove_file(&safe_dest) {
                    summary.items_processed += 1;
                    summary.items_failed += 1;
                    summary.errors.push(format!("{}: {}", entry_name, e));
                    return Ok(true);
                }
            } else if should_skip_existing(request, &entry_name) {
                summary.items_processed += 1;
                send_progress(
                    progress_tx,
                    entry_name,
                    files_completed,
                    total_files,
                    bytes_processed,
                    total_bytes,
                    summary,
                );
                return Ok(true);
            } else {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary
                    .errors
                    .push(format!("{}: destination exists", entry_name));
                return Ok(true);
            }
        }

        if entry.is_directory {
            if let Err(e) = fs::create_dir_all(&safe_dest) {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary.errors.push(format!("{}: {}", entry_name, e));
                return Ok(true);
            }
            files_completed += 1;
            summary.items_processed += 1;
            send_progress(
                progress_tx,
                entry_name,
                files_completed,
                total_files,
                bytes_processed,
                total_bytes,
                summary,
            );
            return Ok(true);
        }

        if let Some(parent) = safe_dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match File::create(&safe_dest) {
            Ok(mut writer) => {
                if let Err(e) = io::copy(reader, &mut writer) {
                    summary.items_processed += 1;
                    summary.items_failed += 1;
                    summary.errors.push(format!("{}: {}", entry_name, e));
                    return Ok(true);
                }
            }
            Err(e) => {
                summary.items_processed += 1;
                summary.items_failed += 1;
                summary.errors.push(format!("{}: {}", entry_name, e));
                return Ok(true);
            }
        }

        files_completed += 1;
        bytes_processed = bytes_processed.saturating_add(entry.size);
        summary.items_processed += 1;
        send_progress(
            progress_tx,
            entry_name,
            files_completed,
            total_files,
            bytes_processed,
            total_bytes,
            summary,
        );
        Ok(true)
    };

    let password = request.password.as_deref();
    let result = if let Some(password) = password {
        sevenz_rust2::decompress_with_extract_fn_and_password(
            file,
            &request.dest_dir,
            SevenZPassword::from(password),
            &mut extract_fn,
        )
    } else {
        sevenz_rust2::decompress_with_extract_fn(file, &request.dest_dir, &mut extract_fn)
    };

    if let Err(error) = result {
        return Err(map_7z_extract_error(&request.archive_path, error, password));
    }

    Ok(())
}

fn list_zip_entries(path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>> {
    let file = File::open(path).map_err(BokslDirError::Io)?;
    let mut archive = ZipArchive::new(file).map_err(|e| map_zip_list_error(path, e, password))?;

    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let entry = match password {
            Some(pass) => archive
                .by_index_decrypt(i, pass.as_bytes())
                .map_err(|e| map_zip_list_error(path, e, password))?,
            None => archive
                .by_index(i)
                .map_err(|e| map_zip_list_error(path, e, password))?,
        };
        entries.push(ArchiveEntry {
            path: entry.name().to_string(),
            size: entry.size(),
            is_dir: entry.is_dir(),
        });
    }
    Ok(entries)
}

fn list_tar_entries(path: &Path) -> Result<Vec<ArchiveEntry>> {
    let file = File::open(path).map_err(BokslDirError::Io)?;
    list_tar_like_entries(TarArchive::new(file), path)
}

fn list_tar_gz_entries(path: &Path) -> Result<Vec<ArchiveEntry>> {
    let file = File::open(path).map_err(BokslDirError::Io)?;
    let decoder = GzDecoder::new(file);
    list_tar_like_entries(TarArchive::new(decoder), path)
}

fn list_tar_zst_entries(path: &Path) -> Result<Vec<ArchiveEntry>> {
    let file = File::open(path).map_err(BokslDirError::Io)?;
    let decoder = ZstdDecoder::new(file).map_err(BokslDirError::Io)?;
    list_tar_like_entries(TarArchive::new(decoder), path)
}

fn list_tar_like_entries<R: Read>(
    mut archive: TarArchive<R>,
    src: &Path,
) -> Result<Vec<ArchiveEntry>> {
    let mut entries = Vec::new();
    for entry_result in archive.entries().map_err(BokslDirError::Io)? {
        let entry = entry_result.map_err(|e| BokslDirError::ArchiveListFailed {
            path: src.to_path_buf(),
            reason: e.to_string(),
        })?;
        let path = entry.path().map_err(|e| BokslDirError::ArchiveListFailed {
            path: src.to_path_buf(),
            reason: e.to_string(),
        })?;
        let path_str = archive_display_path(&path);
        entries.push(ArchiveEntry {
            path: path_str,
            size: entry.size(),
            is_dir: entry.header().entry_type().is_dir(),
        });
    }
    Ok(entries)
}

fn list_7z_entries(path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>> {
    let file = File::open(path).map_err(BokslDirError::Io)?;
    let password_hint = password;
    let password = password
        .map(SevenZPassword::from)
        .unwrap_or_else(SevenZPassword::empty);
    let reader = sevenz_rust2::SevenZReader::new(file, password)
        .map_err(|e| map_7z_list_error(path, e, password_hint))?;

    Ok(reader
        .archive()
        .files
        .iter()
        .map(|e| ArchiveEntry {
            path: e.name.clone(),
            size: e.size,
            is_dir: e.is_directory,
        })
        .collect())
}

fn sanitize_extract_path(dest_root: &Path, raw_path: &Path) -> Option<PathBuf> {
    let mut clean = PathBuf::new();
    for comp in raw_path.components() {
        match comp {
            Component::Normal(v) => clean.push(v),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    let out = dest_root.join(clean);
    if out.starts_with(dest_root) {
        Some(out)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::mpsc;
    use tempfile::tempdir;

    fn progress_tx() -> Sender<ArchiveProgressEvent> {
        let (tx, _rx) = mpsc::channel();
        tx
    }

    fn cancel_flag() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn prepare_sample_sources(base: &Path) -> (PathBuf, PathBuf) {
        let file_path = base.join("alpha.txt");
        let dir_path = base.join("nested");
        fs::create_dir_all(&dir_path).expect("create sample nested dir");
        fs::write(&file_path, b"alpha").expect("write sample file");
        fs::write(dir_path.join("beta.txt"), b"beta").expect("write nested file");
        (file_path, dir_path)
    }

    #[test]
    fn test_detect_archive_format() {
        assert_eq!(
            detect_archive_format(Path::new("/tmp/a.zip")),
            Some(ArchiveFormat::Zip)
        );
        assert_eq!(
            detect_archive_format(Path::new("/tmp/a.jar")),
            Some(ArchiveFormat::Jar)
        );
        assert_eq!(
            detect_archive_format(Path::new("/tmp/a.war")),
            Some(ArchiveFormat::War)
        );
        assert_eq!(
            detect_archive_format(Path::new("/tmp/a.tar")),
            Some(ArchiveFormat::Tar)
        );
        assert_eq!(
            detect_archive_format(Path::new("/tmp/a.tar.gz")),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(
            detect_archive_format(Path::new("/tmp/a.tgz")),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(
            detect_archive_format(Path::new("/tmp/a.tar.zst")),
            Some(ArchiveFormat::TarZst)
        );
        assert_eq!(
            detect_archive_format(Path::new("/tmp/a.tzst")),
            Some(ArchiveFormat::TarZst)
        );
        assert_eq!(
            detect_archive_format(Path::new("/tmp/a.7z")),
            Some(ArchiveFormat::SevenZ)
        );
    }

    #[test]
    fn test_supports_password() {
        assert!(supports_password(ArchiveFormat::Zip));
        assert!(supports_password(ArchiveFormat::SevenZ));
        assert!(!supports_password(ArchiveFormat::Jar));
        assert!(!supports_password(ArchiveFormat::Tar));
    }

    #[test]
    fn test_sanitize_extract_path_blocks_unsafe_paths() {
        let root = PathBuf::from("/tmp/base");
        assert!(sanitize_extract_path(&root, Path::new("ok/file.txt")).is_some());
        assert!(sanitize_extract_path(&root, Path::new("../evil")).is_none());
        assert!(sanitize_extract_path(&root, Path::new("/abs/path")).is_none());
    }

    #[test]
    fn test_zip_create_extract_and_list_roundtrip() {
        let temp = tempdir().expect("create tempdir");
        let src_dir = temp.path().join("src");
        fs::create_dir_all(&src_dir).expect("create src dir");
        let (file_path, nested_dir) = prepare_sample_sources(&src_dir);
        let archive_path = temp.path().join("sample.zip");

        let create_request = ArchiveCreateRequest {
            sources: vec![file_path.clone(), nested_dir.clone()],
            output_path: archive_path.clone(),
            password: None,
        };
        let create_summary = create_archive(&create_request, progress_tx(), cancel_flag())
            .expect("create zip archive");
        assert_eq!(create_summary.items_failed, 0);
        assert!(archive_path.exists());

        let entries = list_entries(&archive_path, None).expect("list zip entries");
        assert!(entries.iter().any(|e| e.path == "alpha.txt"));
        assert!(entries.iter().any(|e| e.path == "nested/beta.txt"));

        let dest = temp.path().join("dest");
        fs::create_dir_all(&dest).expect("create destination dir");
        let extract_request = ArchiveExtractRequest {
            archive_path: archive_path.clone(),
            dest_dir: dest.clone(),
            password: None,
            overwrite_existing: false,
            overwrite_entries: Vec::new(),
            skip_existing_entries: Vec::new(),
            skip_all_existing: false,
        };
        let extract_summary = extract_archive(&extract_request, progress_tx(), cancel_flag())
            .expect("extract zip archive");
        assert_eq!(extract_summary.items_failed, 0);
        assert!(dest.join("alpha.txt").exists());
        assert!(dest.join("nested").join("beta.txt").exists());
    }

    #[test]
    fn test_zip_password_list_and_extract_errors() {
        let temp = tempdir().expect("create tempdir");
        let src_file = temp.path().join("secret.txt");
        fs::write(&src_file, b"top-secret").expect("write source");
        let archive_path = temp.path().join("secret.zip");

        let create_request = ArchiveCreateRequest {
            sources: vec![src_file],
            output_path: archive_path.clone(),
            password: Some("correct-password".to_string()),
        };
        create_archive(&create_request, progress_tx(), cancel_flag())
            .expect("create encrypted zip");

        let no_password = list_entries(&archive_path, None);
        assert!(matches!(
            no_password,
            Err(BokslDirError::ArchivePasswordRequired { .. })
        ));

        let wrong_password = list_entries(&archive_path, Some("wrong-password"));
        assert!(matches!(
            wrong_password,
            Err(BokslDirError::ArchiveInvalidPassword { .. })
        ));

        let with_password = list_entries(&archive_path, Some("correct-password"));
        assert!(with_password.is_ok());

        let wrong_dest = temp.path().join("wrong");
        fs::create_dir_all(&wrong_dest).expect("create wrong dest");
        let wrong_extract = ArchiveExtractRequest {
            archive_path: archive_path.clone(),
            dest_dir: wrong_dest,
            password: Some("wrong-password".to_string()),
            overwrite_existing: false,
            overwrite_entries: Vec::new(),
            skip_existing_entries: Vec::new(),
            skip_all_existing: false,
        };
        let wrong_extract_result = extract_archive(&wrong_extract, progress_tx(), cancel_flag());
        assert!(matches!(
            wrong_extract_result,
            Err(BokslDirError::ArchiveInvalidPassword { .. })
        ));

        let ok_dest = temp.path().join("ok");
        fs::create_dir_all(&ok_dest).expect("create ok dest");
        let ok_extract = ArchiveExtractRequest {
            archive_path: archive_path.clone(),
            dest_dir: ok_dest.clone(),
            password: Some("correct-password".to_string()),
            overwrite_existing: false,
            overwrite_entries: Vec::new(),
            skip_existing_entries: Vec::new(),
            skip_all_existing: false,
        };
        let ok_extract_result = extract_archive(&ok_extract, progress_tx(), cancel_flag());
        assert!(ok_extract_result.is_ok());
        assert!(ok_dest.join("secret.txt").exists());
    }

    #[test]
    fn test_extract_zip_blocks_zip_slip_and_skips_conflicts() {
        let temp = tempdir().expect("create tempdir");
        let archive_path = temp.path().join("unsafe.zip");
        let file = File::create(&archive_path).expect("create zip file");
        let mut writer = ZipWriter::new(file);
        let options = ZipFileOptions::default().compression_method(CompressionMethod::Stored);
        writer
            .start_file("../evil.txt", options)
            .expect("create unsafe entry");
        writer.write_all(b"evil").expect("write unsafe entry");
        writer
            .start_file("safe.txt", options)
            .expect("create safe entry");
        writer.write_all(b"safe").expect("write safe entry");
        writer.finish().expect("finish unsafe zip");

        let dest = temp.path().join("dest");
        fs::create_dir_all(&dest).expect("create dest");
        fs::write(dest.join("safe.txt"), b"keep").expect("create existing conflict file");

        let request = ArchiveExtractRequest {
            archive_path: archive_path.clone(),
            dest_dir: dest.clone(),
            password: None,
            overwrite_existing: false,
            overwrite_entries: Vec::new(),
            skip_existing_entries: Vec::new(),
            skip_all_existing: false,
        };
        let summary = extract_archive(&request, progress_tx(), cancel_flag()).expect("extract zip");

        assert!(summary.items_failed >= 2);
        assert!(summary
            .errors
            .iter()
            .any(|e| e.contains("blocked unsafe path")));
        assert!(summary
            .errors
            .iter()
            .any(|e| e.contains("destination exists")));
        assert!(!temp.path().join("evil.txt").exists());
        assert_eq!(
            fs::read(dest.join("safe.txt")).expect("read conflicted file"),
            b"keep"
        );
    }

    #[test]
    fn test_list_extract_conflicts_detects_existing_paths() {
        let temp = tempdir().expect("create tempdir");
        let src = temp.path().join("sample.txt");
        fs::write(&src, b"payload").expect("write source");
        let archive_path = temp.path().join("sample.zip");
        let create_request = ArchiveCreateRequest {
            sources: vec![src.clone()],
            output_path: archive_path.clone(),
            password: None,
        };
        create_archive(&create_request, progress_tx(), cancel_flag()).expect("create zip");

        let dest = temp.path().join("dest");
        fs::create_dir_all(&dest).expect("create dest");
        fs::write(dest.join("sample.txt"), b"existing").expect("create existing");

        let conflicts =
            list_extract_conflicts(&archive_path, &dest, None).expect("list extract conflicts");
        assert_eq!(conflicts, vec!["sample.txt".to_string()]);
    }

    #[test]
    fn test_extract_zip_overwrite_replaces_existing_file() {
        let temp = tempdir().expect("create tempdir");
        let src = temp.path().join("sample.txt");
        fs::write(&src, b"new-content").expect("write source");
        let archive_path = temp.path().join("sample.zip");
        let create_request = ArchiveCreateRequest {
            sources: vec![src.clone()],
            output_path: archive_path.clone(),
            password: None,
        };
        create_archive(&create_request, progress_tx(), cancel_flag()).expect("create zip");

        let dest = temp.path().join("dest");
        fs::create_dir_all(&dest).expect("create dest");
        fs::write(dest.join("sample.txt"), b"old-content").expect("create existing");

        let request = ArchiveExtractRequest {
            archive_path: archive_path.clone(),
            dest_dir: dest.clone(),
            password: None,
            overwrite_existing: true,
            overwrite_entries: Vec::new(),
            skip_existing_entries: Vec::new(),
            skip_all_existing: false,
        };
        let summary = extract_archive(&request, progress_tx(), cancel_flag()).expect("extract zip");
        assert_eq!(summary.items_failed, 0);
        assert_eq!(
            fs::read(dest.join("sample.txt")).expect("read extracted file"),
            b"new-content"
        );
    }

    #[test]
    fn test_tar_zst_create_extract_roundtrip() {
        let temp = tempdir().expect("create tempdir");
        let src_dir = temp.path().join("src");
        fs::create_dir_all(&src_dir).expect("create src dir");
        let (file_path, nested_dir) = prepare_sample_sources(&src_dir);
        let archive_path = temp.path().join("sample.tar.zst");

        let create_request = ArchiveCreateRequest {
            sources: vec![file_path, nested_dir],
            output_path: archive_path.clone(),
            password: None,
        };
        create_archive(&create_request, progress_tx(), cancel_flag()).expect("create tar.zst");

        let list = list_entries(&archive_path, None).expect("list tar.zst");
        assert!(list.iter().any(|e| e.path == "alpha.txt"));
        assert!(list.iter().any(|e| e.path == "nested/beta.txt"));

        let dest = temp.path().join("dest");
        fs::create_dir_all(&dest).expect("create destination dir");
        let extract_request = ArchiveExtractRequest {
            archive_path,
            dest_dir: dest.clone(),
            password: None,
            overwrite_existing: false,
            overwrite_entries: Vec::new(),
            skip_existing_entries: Vec::new(),
            skip_all_existing: false,
        };
        let summary = extract_archive(&extract_request, progress_tx(), cancel_flag())
            .expect("extract tar.zst");
        assert_eq!(summary.items_failed, 0);
        assert!(dest.join("alpha.txt").exists());
        assert!(dest.join("nested").join("beta.txt").exists());
    }

    #[test]
    fn test_7z_create_extract_roundtrip() {
        let temp = tempdir().expect("create tempdir");
        let src = temp.path().join("plain.txt");
        fs::write(&src, b"plain-7z").expect("write source file");
        let archive_path = temp.path().join("sample.7z");

        let create_request = ArchiveCreateRequest {
            sources: vec![src],
            output_path: archive_path.clone(),
            password: None,
        };
        create_archive(&create_request, progress_tx(), cancel_flag()).expect("create 7z");

        let dest = temp.path().join("dest");
        fs::create_dir_all(&dest).expect("create destination dir");
        let extract_request = ArchiveExtractRequest {
            archive_path,
            dest_dir: dest.clone(),
            password: None,
            overwrite_existing: false,
            overwrite_entries: Vec::new(),
            skip_existing_entries: Vec::new(),
            skip_all_existing: false,
        };
        let summary =
            extract_archive(&extract_request, progress_tx(), cancel_flag()).expect("extract 7z");
        assert_eq!(summary.items_failed, 0);
        assert!(dest.join("plain.txt").exists());
    }

    #[test]
    fn test_create_archive_fails_when_output_exists() {
        let temp = tempdir().expect("create tempdir");
        let src = temp.path().join("plain.txt");
        fs::write(&src, b"plain").expect("write source file");
        let archive_path = temp.path().join("sample.zip");
        fs::write(&archive_path, b"already-exists").expect("pre-create output");

        let create_request = ArchiveCreateRequest {
            sources: vec![src],
            output_path: archive_path.clone(),
            password: None,
        };

        let result = create_archive(&create_request, progress_tx(), cancel_flag());
        assert!(matches!(
            result,
            Err(BokslDirError::ArchiveCreateFailed { path, .. }) if path == archive_path
        ));
    }
}
