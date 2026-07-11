use std::{
    collections::HashSet,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub fn find_external(command: &str) -> Option<PathBuf> {
    let path_var = std::env::var("PATH").ok()?;
    path_var
        .split(':')
        .map(|dir| Path::new(dir).join(command))
        .find(|p| is_executable(p))
}

pub fn find_all_executables(path_var: &str) -> anyhow::Result<HashSet<String>> {
    let set: HashSet<String> = path_var
        .split(':')
        .flat_map(|dir| std::fs::read_dir(dir).into_iter().flatten())
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            is_executable(entry.path()).then_some(name)
        })
        .collect();

    Ok(set)
}

fn is_executable<S>(path: S) -> bool
where
    S: AsRef<Path>,
{
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
