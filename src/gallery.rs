use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::IMAGE_EXTENSIONS;

/// Resolves a path, replacing a leading `~` with the user's home directory.
pub fn resolve_path(path: PathBuf) -> PathBuf {
    let Some(path_str) = path.to_str() else {
        return path;
    };
    if let Some(rest) = path_str.strip_prefix('~') {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let mut resolved = PathBuf::from(home);
        let clean_sub = rest.trim_start_matches(['/', '\\']);
        if !clean_sub.is_empty() {
            resolved.push(clean_sub);
        }
        resolved
    } else {
        path
    }
}

pub fn is_image_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| IMAGE_EXTENSIONS.iter().any(|&e| ext.eq_ignore_ascii_case(e)))
            .unwrap_or(false)
}

/// Scans a directory for image files, sorted case-insensitively by file name.
pub fn scan_dir_images(dir: &Path) -> Vec<PathBuf> {
    let mut images: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_image_file(&path) {
                images.push(path);
            }
        }
    }
    images.sort_by_key(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });
    images
}

pub fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

pub fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "?".to_string())
}

/// Image list for a folder, with a current index for navigation.
pub struct Gallery {
    pub entries: Vec<PathBuf>,
    pub index: usize,
}

impl Gallery {
    /// Builds a gallery from a CLI/startup path (file, directory, or `~/Pictures` default).
    pub fn from_startup_arg(arg: Option<PathBuf>) -> Result<Self, String> {
        let initial_path = resolve_initial_path(arg)?;
        Self::from_image_path(initial_path)
    }

    /// Scans the folder containing `initial` and selects that image.
    pub fn from_image_path(initial: PathBuf) -> Result<Self, String> {
        let dir = initial
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let mut entries = scan_dir_images(&dir);
        if entries.is_empty() {
            entries.push(initial.clone());
        }
        let index = entries
            .iter()
            .position(|p| same_path(p, &initial))
            .unwrap_or(0);
        Ok(Self { entries, index })
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn current(&self) -> Option<&Path> {
        self.entries.get(self.index).map(|p| p.as_path())
    }

    pub fn current_path(&self) -> PathBuf {
        self.entries[self.index].clone()
    }

    /// Advances index cyclically. Returns the new index if navigation happened.
    pub fn next_index(&self) -> Option<usize> {
        if self.entries.len() > 1 {
            Some((self.index + 1) % self.entries.len())
        } else {
            None
        }
    }

    /// Moves index backward cyclically. Returns the new index if navigation happened.
    pub fn prev_index(&self) -> Option<usize> {
        if self.entries.len() > 1 {
            Some((self.index + self.entries.len() - 1) % self.entries.len())
        } else {
            None
        }
    }

    /// Removes the current entry after a successful delete. Returns the removed file name
    /// and whether the gallery is now empty. Index is clamped to a valid next entry.
    pub fn remove_current(&mut self) -> Option<(String, bool)> {
        if self.entries.is_empty() {
            return None;
        }
        let name = file_name_of(&self.entries[self.index]);
        self.entries.remove(self.index);
        let empty = self.entries.is_empty();
        if !empty {
            self.index = self.index.min(self.entries.len() - 1);
        }
        Some((name, empty))
    }

    /// Title fragment: `filename [i/N]` or `(no images)`.
    pub fn title_label(&self) -> String {
        if self.entries.is_empty() {
            "(no images)".to_string()
        } else {
            format!(
                "{} [{}/{}]",
                file_name_of(&self.entries[self.index]),
                self.index + 1,
                self.entries.len()
            )
        }
    }
}

fn resolve_initial_path(arg: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(val) = arg {
        let resolved = resolve_path(val);
        if resolved.is_dir() {
            scan_dir_images(&resolved)
                .into_iter()
                .next()
                .ok_or_else(|| format!("No image files found in directory: {:?}", resolved))
        } else if resolved.is_file() {
            Ok(resolved)
        } else {
            Err(format!(
                "Path does not exist or is not a file/directory: {:?}",
                resolved
            ))
        }
    } else {
        let pictures_dir = resolve_path(PathBuf::from("~/Pictures"));
        if pictures_dir.is_dir() {
            scan_dir_images(&pictures_dir)
                .into_iter()
                .next()
                .ok_or_else(|| {
                    format!(
                        "No image files found in ~/Pictures directory: {:?}",
                        pictures_dir
                    )
                })
        } else {
            Err(format!(
                "~/Pictures directory does not exist: {:?}",
                pictures_dir
            ))
        }
    }
}
