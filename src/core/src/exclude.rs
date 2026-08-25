//! File exclusion filters supporting `.exclude` and `.gitignore`.

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;

/// File exclusion filter supporting both `.exclude` and `.gitignore` patterns.
///
/// Automatically excludes the Kitsu metadata directory (`.kitsu`), the
/// git metadata directory (`.git`), and the Rust build directory (`target`).
/// Additional patterns are loaded from `.exclude` and `.gitignore` files
/// in the repository root.
pub struct Exclude {
    gitignore: Gitignore,
}

impl Exclude {
    /// Loads exclusion patterns from the repository root directory.
    ///
    /// Reads patterns from `.exclude` and `.gitignore` (if present) and
    /// appends built-in exclusions for VCS and build directories.
    pub fn load(root_dir: &Path) -> Self {
        let mut builder = GitignoreBuilder::new(root_dir);

        let exclude_path = root_dir.join(".exclude");
        if exclude_path.exists() {
            builder.add(exclude_path);
        }

        let gitignore_path = root_dir.join(".gitignore");
        if gitignore_path.exists() {
            builder.add(gitignore_path);
        }

        builder.add_line(None, ".kitsu").unwrap();
        builder.add_line(None, ".git").unwrap();
        builder.add_line(None, "target").unwrap();

        Self {
            gitignore: builder.build().unwrap_or_else(|_| Gitignore::empty()),
        }
    }

    /// Tests whether a path should be excluded from tracking.
    ///
    /// The `is_dir` parameter must accurately reflect whether the path
    /// refers to a directory, as some gitignore patterns (trailing `/`)
    /// only match directories.
    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        self.gitignore.matched(path, is_dir).is_ignore()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn exclude_always_ignores_vcontrol_and_build_dirs() {
        let dir = tempdir().unwrap();
        let exclude = Exclude::load(dir.path());
        assert!(exclude.is_ignored(Path::new(".kitsu"), true));
        assert!(exclude.is_ignored(Path::new(".git"), true));
        assert!(exclude.is_ignored(Path::new("target"), true));
        assert!(!exclude.is_ignored(Path::new("src/main.rs"), false));
    }

    #[test]
    fn exclude_respects_both_exclude_and_gitignore_files() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join(".exclude"), "*.log\n").unwrap();
        std::fs::write(dir.path().join(".gitignore"), "*.tmp\n").unwrap();
        let exclude = Exclude::load(dir.path());

        assert!(exclude.is_ignored(Path::new("debug.log"), false));
        assert!(exclude.is_ignored(Path::new("cache.tmp"), false));
        assert!(!exclude.is_ignored(Path::new("valid.rs"), false));
    }
}
