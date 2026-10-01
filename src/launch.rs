//! Host command lookup, independent of package-manager names.
use std::{ffi::OsStr, path::PathBuf};

pub(crate) fn program(name: &str, path: Option<&OsStr>) -> PathBuf {
    #[cfg(windows)]
    {
        use std::path::Path;
        if Path::new(name).components().count() == 1 && Path::new(name).extension().is_none() {
            let inherited = std::env::var_os("PATH");
            let extensions =
                std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
            let extensions: Vec<_> = extensions
                .split(';')
                .map(|part| part.trim_start_matches('.').to_ascii_lowercase())
                .filter(|part| matches!(part.as_str(), "exe" | "com" | "bat" | "cmd"))
                .collect();
            if let Some(path) = path.or(inherited.as_deref()) {
                for directory in std::env::split_paths(path) {
                    for extension in &extensions {
                        let candidate = directory.join(format!("{name}.{extension}"));
                        if candidate.is_file() {
                            return candidate;
                        }
                    }
                }
            }
        }
    }
    #[cfg(not(windows))]
    let _ = path;
    name.into()
}
