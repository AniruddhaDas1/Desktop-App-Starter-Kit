// Port of src/main/services/fs-service.ts.
//
// Security model, unchanged from upstream:
//   1. Anything under app_data_dir (Electron's userData) is always allowed.
//   2. Paths the user picked in a native dialog or dropped onto the window are
//      "blessed"; their children are allowed too.
//
// Symlinks are resolved before the allow-list check, so a blessed directory
// cannot be used to reach /etc/passwd through a planted link. For writes to a
// file that does not exist yet we canonicalize the parent and rejoin the name.
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

const MAX_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FsReadRequest {
    pub path: String,
    pub encoding: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FsWriteRequest {
    pub path: String,
    pub content: String,
    pub encoding: Option<String>,
    pub exclusive: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FsPathRequest {
    pub path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsReadResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsWriteResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_written: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsListEntry {
    pub name: String,
    pub is_file: bool,
    pub is_directory: bool,
    pub size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsListResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<FsListEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsStatResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_file: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_directory: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DndResolvedPath {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_file: bool,
    pub is_directory: bool,
    pub exists: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DndResolvePathsResponse {
    pub success: bool,
    pub paths: Vec<DndResolvedPath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DndBlessAndResolveResponse {
    pub success: bool,
    pub resolved: DndResolvedPath,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DndResolvePathsRequest {
    pub paths: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DndBlessAndResolveRequest {
    pub path: String,
}

// ---------- path helpers ----------

fn normalize(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(path.to_string_lossy().to_lowercase())
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

fn resolve(raw: &str) -> PathBuf {
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        return normalize(&candidate);
    }
    match std::env::current_dir() {
        Ok(cwd) => normalize(&cwd.join(candidate)),
        Err(_) => normalize(&candidate),
    }
}

/// Resolve symlinks before checking the allow-list. Falls back to the parent's
/// canonical path for files that do not exist yet.
fn canonicalize(path: &Path) -> Option<PathBuf> {
    match fs::canonicalize(path) {
        Ok(real) => Some(normalize(&real)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent()?;
            let base = path.file_name()?;
            match fs::canonicalize(parent) {
                Ok(real_parent) => Some(normalize(&real_parent.join(base))),
                Err(_) => Some(normalize(path)),
            }
        }
        Err(_) => None,
    }
}

fn is_within(parent: &Path, child: &Path) -> bool {
    child == parent || child.starts_with(parent)
}

pub fn is_allowed(state: &AppState, path: &Path) -> bool {
    let path = normalize(path);
    if is_within(&normalize(&state.app_data_dir), &path) {
        return true;
    }
    let blessed = match state.blessed.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    blessed.iter().any(|root| is_within(root, &path))
}

/// Canonicalize a raw path first, then check it. Used by commands that receive
/// a path string directly from the renderer (`app:open-path`, `app:show-in-folder`).
pub fn is_path_allowed(state: &AppState, raw: &str) -> bool {
    match canonicalize(&resolve(raw)) {
        Some(real) => is_allowed(state, &real),
        None => false,
    }
}

pub fn bless(state: &AppState, path: &Path) {
    let target = canonicalize(path).unwrap_or_else(|| normalize(path));
    let mut blessed = match state.blessed.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    blessed.insert(target);
}

fn resolved_entry(path: &Path) -> DndResolvedPath {
    let display = path.to_string_lossy().to_string();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| display.clone());

    match fs::metadata(path) {
        Ok(meta) => DndResolvedPath {
            path: display,
            name,
            size: if meta.is_file() { meta.len() } else { 0 },
            is_file: meta.is_file(),
            is_directory: meta.is_dir(),
            exists: true,
        },
        Err(_) => DndResolvedPath {
            path: display,
            name,
            size: 0,
            is_file: false,
            is_directory: false,
            exists: false,
        },
    }
}

// ---------- commands ----------

#[tauri::command]
pub fn fs_read(state: State<'_, AppState>, data: FsReadRequest) -> FsReadResponse {
    let abs = resolve(&data.path);
    let real = match canonicalize(&abs) {
        Some(real) => real,
        None => return fs_read_error(AppError::forbidden()),
    };

    if !is_allowed(&state, &real) {
        return fs_read_error(AppError::forbidden());
    }

    let meta = match fs::metadata(&real) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return fs_read_error(AppError::with_code("File not found", "not-found"))
        }
        Err(err) => return fs_read_error(AppError::new(err.to_string())),
    };

    if !meta.is_file() {
        return fs_read_error(AppError::with_code("Not a regular file", "not-a-file"));
    }

    if meta.len() > MAX_BYTES {
        return fs_read_error(AppError::with_code(
            format!("File exceeds {MAX_BYTES} bytes (size {})", meta.len()),
            "too-large",
        ));
    }

    let bytes = match fs::read(&real) {
        Ok(bytes) => bytes,
        Err(err) => return fs_read_error(AppError::new(err.to_string())),
    };

    let encoding = data.encoding.unwrap_or_else(|| "utf8".into());
    let content = if encoding == "base64" {
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };

    FsReadResponse {
        success: true,
        content: Some(content),
        encoding: Some(encoding),
        size: Some(meta.len()),
        error: None,
        code: None,
    }
}

fn fs_read_error(err: AppError) -> FsReadResponse {
    FsReadResponse {
        success: false,
        content: None,
        encoding: None,
        size: None,
        error: Some(err.error),
        code: err.code,
    }
}

#[tauri::command]
pub fn fs_write(state: State<'_, AppState>, data: FsWriteRequest) -> FsWriteResponse {
    let abs = resolve(&data.path);
    let real = match canonicalize(&abs) {
        Some(real) => real,
        None => return fs_write_error(AppError::forbidden()),
    };

    if !is_allowed(&state, &real) {
        return fs_write_error(AppError::forbidden());
    }

    let encoding = data.encoding.unwrap_or_else(|| "utf8".into());
    let bytes = if encoding == "base64" {
        match base64::engine::general_purpose::STANDARD.decode(data.content.as_bytes()) {
            Ok(bytes) => bytes,
            Err(err) => return fs_write_error(AppError::new(format!("Invalid base64: {err}"))),
        }
    } else {
        data.content.into_bytes()
    };

    if bytes.len() as u64 > MAX_BYTES {
        return fs_write_error(AppError::with_code(
            format!("Content exceeds {MAX_BYTES} bytes (got {})", bytes.len()),
            "too-large",
        ));
    }

    if let Some(parent) = real.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return fs_write_error(AppError::new(err.to_string()));
        }
    }

    let mut options = fs::OpenOptions::new();
    options.write(true);
    if data.exclusive.unwrap_or(false) {
        options.create_new(true);
    } else {
        options.create(true).truncate(true);
    }

    let mut file = match options.open(&real) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            return fs_write_error(AppError::with_code("File already exists", "exists"))
        }
        Err(err) => return fs_write_error(AppError::new(err.to_string())),
    };

    if let Err(err) = file.write_all(&bytes) {
        return fs_write_error(AppError::new(err.to_string()));
    }

    FsWriteResponse {
        success: true,
        bytes_written: Some(bytes.len()),
        error: None,
        code: None,
    }
}

fn fs_write_error(err: AppError) -> FsWriteResponse {
    FsWriteResponse {
        success: false,
        bytes_written: None,
        error: Some(err.error),
        code: err.code,
    }
}

#[tauri::command]
pub fn fs_list(state: State<'_, AppState>, data: FsPathRequest) -> FsListResponse {
    let abs = resolve(&data.path);
    let real = match canonicalize(&abs) {
        Some(real) => real,
        None => return fs_list_error(AppError::forbidden()),
    };

    if !is_allowed(&state, &real) {
        return fs_list_error(AppError::forbidden());
    }

    let meta = match fs::metadata(&real) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return fs_list_error(AppError::with_code("Directory not found", "not-found"))
        }
        Err(err) => return fs_list_error(AppError::new(err.to_string())),
    };

    if !meta.is_dir() {
        return fs_list_error(AppError::with_code("Not a directory", "not-a-directory"));
    }

    let read_dir = match fs::read_dir(&real) {
        Ok(read_dir) => read_dir,
        Err(err) => return fs_list_error(AppError::new(err.to_string())),
    };

    let mut entries = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        let is_directory = meta.is_dir();
        let size = if meta.is_file() { meta.len() } else { 0 };
        entries.push(FsListEntry {
            name,
            is_file: meta.is_file(),
            is_directory,
            size,
        });
    }

    FsListResponse {
        success: true,
        entries: Some(entries),
        error: None,
        code: None,
    }
}

fn fs_list_error(err: AppError) -> FsListResponse {
    FsListResponse {
        success: false,
        entries: None,
        error: Some(err.error),
        code: err.code,
    }
}

#[tauri::command]
pub fn fs_stat(state: State<'_, AppState>, data: FsPathRequest) -> FsStatResponse {
    let abs = resolve(&data.path);
    let real = match canonicalize(&abs) {
        Some(real) => real,
        None => return fs_stat_error(AppError::forbidden()),
    };

    if !is_allowed(&state, &real) {
        return fs_stat_error(AppError::forbidden());
    }

    match fs::metadata(&real) {
        Ok(meta) => {
            let mtime_ms = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            FsStatResponse {
                success: true,
                exists: Some(true),
                is_file: Some(meta.is_file()),
                is_directory: Some(meta.is_dir()),
                size: Some(meta.len()),
                mtime_ms: Some(mtime_ms),
                error: None,
                code: None,
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => FsStatResponse {
            success: true,
            exists: Some(false),
            is_file: Some(false),
            is_directory: Some(false),
            size: Some(0),
            mtime_ms: Some(0),
            error: None,
            code: None,
        },
        Err(err) => fs_stat_error(AppError::new(err.to_string())),
    }
}

fn fs_stat_error(err: AppError) -> FsStatResponse {
    FsStatResponse {
        success: false,
        exists: None,
        is_file: None,
        is_directory: None,
        size: None,
        mtime_ms: None,
        error: Some(err.error),
        code: err.code,
    }
}

#[tauri::command]
pub fn dnd_resolve_paths(data: DndResolvePathsRequest) -> DndResolvePathsResponse {
    let paths = data
        .paths
        .iter()
        .map(|raw| resolved_entry(&resolve(raw)))
        .collect();
    DndResolvePathsResponse {
        success: true,
        paths,
        error: None,
    }
}

#[tauri::command]
pub fn dnd_bless_and_resolve(
    state: State<'_, AppState>,
    data: DndBlessAndResolveRequest,
) -> DndBlessAndResolveResponse {
    if !Path::new(&data.path).is_absolute() {
        return DndBlessAndResolveResponse {
            success: false,
            resolved: DndResolvedPath {
                path: String::new(),
                name: String::new(),
                size: 0,
                is_file: false,
                is_directory: false,
                exists: false,
            },
            error: Some("Path must be absolute".into()),
        };
    }

    let real = canonicalize(&resolve(&data.path)).unwrap_or_else(|| resolve(&data.path));
    let resolved = resolved_entry(&real);

    // Only grant access to paths that actually exist, so a drop cannot
    // pre-bless a location that appears later.
    if resolved.exists {
        bless(&state, &real);
    }

    DndBlessAndResolveResponse {
        success: true,
        resolved,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_within_accepts_the_root_and_children_only() {
        let root = Path::new("/tmp/app/data");
        assert!(is_within(root, root));
        assert!(is_within(root, Path::new("/tmp/app/data/file.txt")));
        assert!(is_within(root, Path::new("/tmp/app/data/nested/deep.txt")));
        // A sibling whose name merely starts with the root's name must fail.
        assert!(!is_within(root, Path::new("/tmp/app/database")));
        assert!(!is_within(root, Path::new("/tmp/other/file.txt")));
    }

    #[test]
    fn canonicalize_handles_files_that_do_not_exist_yet() {
        let dir = std::env::temp_dir().join(format!("das-fs-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let target = dir.join("not-created.txt");
        let real = canonicalize(&target).expect("the parent directory exists");
        assert!(real.ends_with("not-created.txt"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_returns_an_absolute_path() {
        assert!(resolve("relative.txt").is_absolute());
        assert!(resolve("/tmp/absolute.txt").is_absolute());
    }

    #[test]
    fn resolved_entry_reports_missing_files_without_failing() {
        let missing = std::env::temp_dir().join("das-definitely-missing-file");
        let entry = resolved_entry(&missing);
        assert!(!entry.exists);
        assert!(!entry.is_file);
        assert!(!entry.is_directory);
    }
}

#[cfg(test)]
mod allow_list_tests {
    use super::*;
    use crate::state::{AppSettings, AppState};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("das-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn allow_list_covers_app_data_and_blessed_paths_only() {
        let root = temp_dir("allow-list");
        let app_data = root.join("appdata");
        let picked = root.join("picked");
        let outside = root.join("outside");
        for dir in [&app_data, &picked, &outside] {
            fs::create_dir_all(dir).unwrap();
        }

        let app_data_file = app_data.join("inside.txt");
        let picked_file = picked.join("picked.txt");
        let outside_file = outside.join("outside.txt");
        for file in [&app_data_file, &picked_file, &outside_file] {
            fs::write(file, b"x").unwrap();
        }

        let state = AppState::new(app_data.clone(), AppSettings::default());

        assert!(is_path_allowed(&state, app_data_file.to_str().unwrap()));
        assert!(!is_path_allowed(&state, picked_file.to_str().unwrap()));
        assert!(!is_path_allowed(&state, outside_file.to_str().unwrap()));

        bless(&state, &picked);

        assert!(is_path_allowed(&state, picked_file.to_str().unwrap()));
        assert!(!is_path_allowed(&state, outside_file.to_str().unwrap()));
        assert!(!is_path_allowed(&state, "/etc/passwd"));

        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_escape_a_blessed_folder() {
        let root = temp_dir("allow-symlink");
        let picked = root.join("picked");
        let outside = root.join("outside");
        fs::create_dir_all(&picked).unwrap();
        fs::create_dir_all(&outside).unwrap();

        let outside_file = outside.join("secret.txt");
        fs::write(&outside_file, b"secret").unwrap();

        let link = picked.join("escape.txt");
        std::os::unix::fs::symlink(&outside_file, &link).unwrap();

        let state = AppState::new(root.join("appdata"), AppSettings::default());
        bless(&state, &picked);

        // The link resolves to a file outside the blessed folder.
        assert!(!is_path_allowed(&state, link.to_str().unwrap()));

        let _ = fs::remove_dir_all(&root);
    }
}
