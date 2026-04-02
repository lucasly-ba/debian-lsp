use super::detection::list_patch_files;
use std::collections::HashSet;

use tower_lsp_server::ls_types::{CompletionItem, CompletionItemKind, Position, Uri};

/// Get completion items for a debian/patches/series file
pub fn get_completions(
    _uri: &Uri,
    parsed: &patchkit::quilt::Series,
    source_text: &str,
    position: Position,
) -> Vec<CompletionItem> {
    let current_line = source_text
        .lines()
        .nth(position.line as usize)
        .unwrap_or("");

    let before_cursor = &current_line[..position.character as usize];

    let tokens: Vec<&str> = before_cursor.split_whitespace().collect();

    // If we already have two tokens, we don't want more completions.
    if tokens.len() >= 2 {
        return Vec::new();
    }

    if before_cursor.ends_with(' ') {
        return get_strip_option_completions();
    }

    // Skip patches that are already listed in the series.
    let already_listed: HashSet<String> = parsed
        .entries
        .iter()
        .filter_map(|e| match e {
            patchkit::quilt::SeriesEntry::Patch { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();

    let patch_files = list_patch_files(_uri);

    get_patch_file_completions(&patch_files, &already_listed)
}

// Get snippet completions for each patches in the debian/patches folder
fn get_patch_file_completions(
    patch_files: &HashSet<String>,
    already_listed: &HashSet<String>,
) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = patch_files
        .iter()
        .filter(|name| !already_listed.contains(*name))
        .map(|name| CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::FILE),
            detail: Some("Patch file".to_string()),
            ..Default::default()
        })
        .collect();

    items.sort_by(|a, b| a.label.cmp(&b.label));
    items
}

/// Get completion items for the strip option (-p0, -p1, -p2)
fn get_strip_option_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "-p0".to_string(),
            kind: Some(CompletionItemKind::VALUE),
            detail: Some("No path stripping".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "-p1".to_string(),
            kind: Some(CompletionItemKind::VALUE),
            detail: Some("Strip 1 path component (default)".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "-p2".to_string(),
            kind: Some(CompletionItemKind::VALUE),
            detail: Some("Strip 2 path components".to_string()),
            ..Default::default()
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchkit::quilt::{Series, SeriesEntry};

    fn make_series(patches: &[&str]) -> Series {
        Series {
            entries: patches
                .iter()
                .map(|name| SeriesEntry::Patch {
                    name: name.to_string(),
                    options: vec![],
                })
                .collect(),
        }
    }

    fn empty_series() -> Series {
        Series { entries: vec![] }
    }

    #[test]
    fn test_strip_option_completions_after_space() {
        let series = make_series(&["fix-arm.patch"]);
        let source_text = "fix-arm.patch ";
        let position = Position::new(0, 14);
        let uri: Uri = "file:///debian/patches/series".parse().unwrap();

        let completions = get_completions(&uri, &series, source_text, position);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();

        assert_eq!(labels, vec!["-p0", "-p1", "-p2"]);
        assert!(completions
            .iter()
            .all(|c| c.kind == Some(CompletionItemKind::VALUE)));
    }

    #[test]
    fn test_strip_option_completions_have_details() {
        let series = make_series(&["fix-arm.patch"]);
        let source_text = "fix-arm.patch ";
        let position = Position::new(0, 14);
        let uri: Uri = "file:///debian/patches/series".parse().unwrap();

        let completions = get_completions(&uri, &series, source_text, position);

        let p0 = completions.iter().find(|c| c.label == "-p0").unwrap();
        let p1 = completions.iter().find(|c| c.label == "-p1").unwrap();
        let p2 = completions.iter().find(|c| c.label == "-p2").unwrap();

        assert_eq!(p0.detail.as_deref(), Some("No path stripping"));
        assert_eq!(
            p1.detail.as_deref(),
            Some("Strip 1 path component (default)")
        );
        assert_eq!(p2.detail.as_deref(), Some("Strip 2 path components"));
    }

    #[test]
    fn test_no_completions_when_patch_and_option_present() {
        let series = make_series(&["fix-arm.patch"]);
        let source_text = "fix-arm.patch -p1 ";
        let position = Position::new(0, 18);
        let uri: Uri = "file:///debian/patches/series".parse().unwrap();

        let completions = get_completions(&uri, &series, source_text, position);

        assert!(completions.is_empty());
    }

    #[test]
    fn test_no_completions_when_two_tokens() {
        let series = make_series(&["fix-arm.patch"]);
        let source_text = "fix-arm.patch -p1";
        let position = Position::new(0, 17);
        let uri: Uri = "file:///debian/patches/series".parse().unwrap();

        let completions = get_completions(&uri, &series, source_text, position);

        assert!(completions.is_empty());
    }

    #[test]
    fn test_patch_file_completions_excludes_already_listed() {
        let patch_files: HashSet<String> = vec![
            "fix-arm.patch".to_string(),
            "fix-mips.patch".to_string(),
            "CVE-2024.patch".to_string(),
        ]
        .into_iter()
        .collect();

        let already_listed: HashSet<String> =
            vec!["fix-arm.patch".to_string()].into_iter().collect();

        let completions = get_patch_file_completions(&patch_files, &already_listed);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();

        assert!(!labels.contains(&"fix-arm.patch"));
        assert!(labels.contains(&"fix-mips.patch"));
        assert!(labels.contains(&"CVE-2024.patch"));
    }

    #[test]
    fn test_patch_file_completions_sorted() {
        let patch_files: HashSet<String> = vec![
            "zzz.patch".to_string(),
            "aaa.patch".to_string(),
            "mmm.patch".to_string(),
        ]
        .into_iter()
        .collect();

        let already_listed: HashSet<String> = HashSet::new();

        let completions = get_patch_file_completions(&patch_files, &already_listed);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();

        assert_eq!(labels, vec!["aaa.patch", "mmm.patch", "zzz.patch"]);
    }

    #[test]
    fn test_patch_file_completions_have_file_kind() {
        let patch_files: HashSet<String> = vec!["fix-arm.patch".to_string()].into_iter().collect();

        let already_listed: HashSet<String> = HashSet::new();

        let completions = get_patch_file_completions(&patch_files, &already_listed);

        assert!(completions
            .iter()
            .all(|c| c.kind == Some(CompletionItemKind::FILE)));
    }

    #[test]
    fn test_patch_file_completions_all_listed_returns_empty() {
        let patch_files: HashSet<String> =
            vec!["fix-arm.patch".to_string(), "fix-mips.patch".to_string()]
                .into_iter()
                .collect();

        let already_listed = patch_files.clone();

        let completions = get_patch_file_completions(&patch_files, &already_listed);

        assert!(completions.is_empty());
    }

    #[test]
    fn test_get_strip_option_completions_count() {
        let completions = get_strip_option_completions();
        assert_eq!(completions.len(), 3);
    }

    #[test]
    fn test_get_strip_option_completions_labels() {
        let completions = get_strip_option_completions();
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, vec!["-p0", "-p1", "-p2"]);
    }

    #[test]
    fn test_no_strip_options_without_space() {
        // Si on est en train de taper le nom du patch, pas d'espace → pas de -p
        let series = empty_series();
        let source_text = "fix-arm";
        let position = Position::new(0, 7);
        let uri: Uri = "file:///debian/patches/series".parse().unwrap();

        let completions = get_completions(&uri, &series, source_text, position);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();

        assert!(!labels.contains(&"-p0"));
        assert!(!labels.contains(&"-p1"));
        assert!(!labels.contains(&"-p2"));
    }
}
