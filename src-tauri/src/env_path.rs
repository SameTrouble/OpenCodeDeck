use std::path::PathBuf;

const EXTRA_PATH_DIRS_MACOS: &[&str] = &[
    "/opt/homebrew/bin",
    "/opt/homebrew/sbin",
    "/usr/local/bin",
    "/usr/local/sbin",
    "~/.bun/bin",
    "~/.opencode/bin",
    "~/.npm-global/bin",
    "~/.local/bin",
    "~/.cargo/bin",
    "~/go/bin",
    "~/.nvm/versions/node",
];

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

fn expand_tilde(s: &str) -> Option<PathBuf> {
    if let Some(rest) = s.strip_prefix("~/") {
        home_dir().map(|h| h.join(rest))
    } else if s == "~" {
        home_dir()
    } else {
        Some(PathBuf::from(s))
    }
}

fn extra_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for raw in EXTRA_PATH_DIRS_MACOS {
        if let Some(p) = expand_tilde(raw) {
            if p.is_dir() {
                out.push(p);
            }
        }
    }
    if cfg!(target_os = "macos") {
        if let Some(home) = home_dir() {
            let nvm_versions = home.join(".nvm/versions/node");
            if let Ok(entries) = std::fs::read_dir(&nvm_versions) {
                for entry in entries.flatten() {
                    let bin = entry.path().join("bin");
                    if bin.is_dir() {
                        out.push(bin);
                    }
                }
            }
        }
    }
    out
}

pub fn augment_path() {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut parts: Vec<PathBuf> = std::env::split_paths(&current).collect();
    for p in extra_paths() {
        if !parts.contains(&p) {
            parts.push(p);
        }
    }
    let new_path = std::env::join_paths(parts).unwrap_or(current);
    std::env::set_var("PATH", new_path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_home() {
        let p = expand_tilde("~/foo").unwrap();
        assert!(p.starts_with(dirs::home_dir().unwrap()));
    }

    #[test]
    fn expand_tilde_absolute() {
        let p = expand_tilde("/usr/local/bin").unwrap();
        assert_eq!(p, PathBuf::from("/usr/local/bin"));
    }

    #[test]
    fn extra_paths_filters_nonexistent() {
        let paths = extra_paths();
        for p in &paths {
            assert!(p.is_dir(), "{} should exist", p.display());
        }
    }
}
