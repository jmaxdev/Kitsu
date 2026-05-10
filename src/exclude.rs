use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;

pub struct Exclude {
    gitignore: Gitignore,
}

impl Exclude {
    pub fn load(root_dir: &Path) -> Self {
        let exclude_path = root_dir.join(".exclude");
        let mut builder = GitignoreBuilder::new(root_dir);
        
        if exclude_path.exists() {
            builder.add(exclude_path);
        }
        
        // Siempre ignorar la carpeta del VCS y otras carpetas comunes de sistema
        // Obtenemos el nombre de la carpeta desde la configuración inyectada
        let vcs_dir = crate::config::DIR_NAME;
        builder.add_line(None, vcs_dir).unwrap();
        builder.add_line(None, ".git").unwrap();
        builder.add_line(None, "target").unwrap();
        
        Self {
            gitignore: builder.build().unwrap_or_else(|_| Gitignore::empty()),
        }
    }

    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        self.gitignore.matched(path, is_dir).is_ignore()
    }
}
