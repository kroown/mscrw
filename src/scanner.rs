use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;
use serde::Serialize;

use crate::image;
use crate::text;

pub struct ScannedFile {
    pub path: PathBuf,
    pub name: String,
    pub extension: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileMetadata {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: u64,
    pub permissions: String,
    pub permissions_octal: u32,
    pub owner: String,
    pub group: String,
    pub created: u64,
    pub modified: u64,
    pub accessed: u64,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub is_system: bool,
    pub is_readonly: bool,
    pub image: Option<image::ImageMeta>,
    pub text: Option<text::TextMeta>,
}

pub fn scan_all(paths: &[PathBuf], verbose: bool) -> Vec<ScannedFile> {
    let mut files = Vec::new();
    for p in paths {
        if p.is_dir() {
            for entry in WalkDir::new(p).sort_by(|a, b| a.file_name().cmp(b.file_name())) {
                match entry {
                    Ok(e) => {
                        let path = e.path().to_path_buf();
                        let name = e.file_name().to_string_lossy().to_string();
                        let ext = path.extension()
                            .map(|s| format!(".{}", s.to_string_lossy()))
                            .unwrap_or_default();
                        files.push(ScannedFile { path, name, extension: ext, is_dir: e.file_type().is_dir() });
                    }
                    Err(_) => continue,
                }
            }
        } else if p.exists() {
            let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let ext = p.extension().map(|s| format!(".{}", s.to_string_lossy())).unwrap_or_default();
            files.push(ScannedFile { path: p.clone(), name, extension: ext, is_dir: false });
        } else if verbose {
            eprintln!("mcsrw: skipping {}", p.display());
        }
    }
    files
}

#[cfg(windows)]
fn get_windows_attrs(path: &Path) -> (bool, bool, bool) {
    use std::os::windows::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|m| {
        let attrs = m.file_attributes();
        (attrs & 0x2 != 0, attrs & 0x4 != 0, attrs & 0x1 != 0)
    }).unwrap_or((false, false, false))
}

#[cfg(not(windows))]
fn get_windows_attrs(_path: &Path) -> (bool, bool, bool) {
    (false, false, false)
}

#[cfg(windows)]
fn get_creation_time(path: &Path) -> u64 {
    use std::os::windows::fs::MetadataExt;
    std::fs::metadata(path).ok()
        .map(|m| {
            let filetime = m.creation_time();
            let epoch_diff = 116_444_736_00u64.saturating_mul(10_000_000);
            filetime.saturating_sub(epoch_diff).saturating_mul(100)
        })
        .unwrap_or(0)
}

#[cfg(not(windows))]
fn get_creation_time(path: &Path) -> u64 {
    std::fs::metadata(path).ok()
        .and_then(|m| m.created().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(unix)]
fn get_unix_owner(path: &Path) -> (String, String) {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|m| {
        (m.uid().to_string(), m.gid().to_string())
    }).unwrap_or_else(|| ("0".into(), "0".into()))
}

#[cfg(not(unix))]
fn get_unix_owner(_path: &Path) -> (String, String) {
    ("-".into(), "-".into())
}

#[allow(unused_variables)]
fn format_permissions(path: &Path, is_dir: bool) -> (String, u32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(m) = std::fs::symlink_metadata(path) {
            let bits = m.permissions().mode() & 0o777;
            let s = format!(
                "{}{}{}{}{}{}{}{}{}{}",
                if is_dir { 'd' } else { '-' },
                if bits & 0o400 != 0 { 'r' } else { '-' },
                if bits & 0o200 != 0 { 'w' } else { '-' },
                if bits & 0o100 != 0 { 'x' } else { '-' },
                if bits & 0o040 != 0 { 'r' } else { '-' },
                if bits & 0o020 != 0 { 'w' } else { '-' },
                if bits & 0o010 != 0 { 'x' } else { '-' },
                if bits & 0o004 != 0 { 'r' } else { '-' },
                if bits & 0o002 != 0 { 'w' } else { '-' },
                if bits & 0o001 != 0 { 'x' } else { '-' },
            );
            return (s, bits as u32);
        }
    }
    (String::new(), 0)
}

fn is_text_ext(ext: &str) -> bool {
    matches!(ext, ".txt" | ".md" | ".csv" | ".xml" | ".html" | ".htm"
        | ".json" | ".yaml" | ".yml" | ".toml" | ".ini"
        | ".cfg" | ".conf" | ".sh" | ".py" | ".js"
        | ".css" | ".cpp" | ".c" | ".hpp" | ".h"
        | ".rs" | ".go" | ".java" | ".ts" | ".tsx"
        | ".s" | ".asm" | ".zig" | ".tex" | ".bib"
        | ".log" | ".env" | ".gitignore" | ".dockerignore")
}

pub fn collect_one(file: &ScannedFile) -> FileMetadata {
    let path = &file.path;
    let meta = std::fs::symlink_metadata(path);
    let is_symlink = meta.as_ref().map(|m| m.file_type().is_symlink()).unwrap_or(false);

    let (size, is_dir) = meta.as_ref()
        .map(|m| (m.len(), m.is_dir()))
        .unwrap_or((0, file.is_dir));

    let (perms, perm_oct) = format_permissions(path, is_dir);
    let (owner, group) = get_unix_owner(path);
    let (hidden, system, readonly) = get_windows_attrs(path);
    let created = get_creation_time(path);

    let modified = meta.as_ref().ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    let accessed = meta.as_ref().ok()
        .and_then(|m| m.accessed().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    let image = if !is_dir {
        match file.extension.to_lowercase().as_str() {
            ".jpg" | ".jpeg" | ".png" | ".gif" | ".bmp" | ".webp" => {
                image::parse(std::fs::read(path).ok().as_deref())
            }
            _ => None,
        }
    } else {
        None
    };

    let text = if !is_dir && is_text_ext(&file.extension) {
        text::analyze(std::fs::read(path).ok().as_deref())
    } else {
        None
    };

    FileMetadata {
        path: path.to_string_lossy().to_string(),
        name: file.name.clone(),
        extension: file.extension.clone(),
        size,
        permissions: perms,
        permissions_octal: perm_oct,
        owner,
        group,
        created,
        modified,
        accessed,
        is_directory: is_dir,
        is_symlink,
        is_hidden: hidden,
        is_system: system,
        is_readonly: readonly,
        image,
        text,
    }
}

pub fn collect_all(files: Vec<ScannedFile>, threads: usize, verbose: bool) -> Vec<FileMetadata> {
    let n = files.len();
    if n == 0 {
        return Vec::new();
    }
    let files = Arc::new(files);
    let results = Arc::new(Mutex::new(vec![None::<FileMetadata>; n]));
    let next = Arc::new(Mutex::new(0usize));
    let thread_count = threads.min(n).max(1);
    let mut handles = Vec::new();

    for _ in 0..thread_count {
        let files = Arc::clone(&files);
        let results = Arc::clone(&results);
        let next = Arc::clone(&next);
        handles.push(std::thread::spawn(move || loop {
            let idx = {
                let mut n = next.lock().unwrap();
                if *n >= files.len() { return; }
                let i = *n;
                *n += 1;
                i
            };
            let meta = collect_one(&files[idx]);
            if verbose && idx % 100 == 0 {
                eprintln!("mcsrw: {}/{}", idx, files.len());
            }
            results.lock().unwrap()[idx] = Some(meta);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let mut out = Vec::with_capacity(n);
    for r in results.lock().unwrap().iter_mut() {
        if let Some(m) = r.take() {
            out.push(m);
        }
    }
    out
}
