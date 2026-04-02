use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tower_lsp_server::ls_types::Uri;

/// Check if a given URL represents a Debian patches/series file
pub fn is_patches_series_file(uri: &Uri) -> bool {
    let path = uri.as_str();
    path.ends_with("/debian/patches/series")
}

// Get all patch files in a debian/patches folder
pub fn list_patch_files(uri: &Uri) -> HashSet<String> {
    let Some(path) = uri.to_file_path() else {
        return HashSet::new();
    };
    let Some(patches_dir) = path.parent() else {
        return HashSet::new();
    };
    let patches_dir = patches_dir.to_path_buf();
    let mut result = HashSet::new();
    collect_patches(&patches_dir, &patches_dir, &mut result);
    result
}

fn collect_patches(base: &Path, dir: &Path, result: &mut HashSet<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                collect_patches(base, &path, result);
            } else if path.extension().and_then(|e| e.to_str()) == Some("patch") {
                if let Ok(relative) = path.strip_prefix(base) {
                    if let Some(s) = relative.to_str() {
                        result.insert(s.to_string());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_patches_series_file() {
        let tests_patches_series_paths = vec![
            "file:///path/to/debian/patches/series",
            "file:///project/debian/patches/series",
        ];

        let non_tests_patches_series_paths = vec![
            "file:///path/to/other.txt",
            "file:///path/to/debian/control",
            "file:///path/to/debian/copyright",
            "file:///path/to/debian/watch",
            "file:///path/to/patches/series", // Not in debian/ directory
            "file:///path/to/debian/tests/control.backup",
        ];

        for path in tests_patches_series_paths {
            let uri = path.parse::<Uri>().unwrap();
            assert!(
                is_patches_series_file(&uri),
                "Should detect tests/patches/series file: {}",
                path
            );
        }

        for path in non_tests_patches_series_paths {
            let uri = path.parse::<Uri>().unwrap();
            assert!(
                !is_patches_series_file(&uri),
                "Should not detect as tests/patches/series file: {}",
                path
            );
        }
    }
}
