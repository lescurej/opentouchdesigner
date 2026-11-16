use std::path::{Path, PathBuf};
use std::sync::Once;

static CONFIGURED: Once = Once::new();

pub fn configure_bundled_python() {
    CONFIGURED.call_once(configure_bundled_python_once);
}

fn configure_bundled_python_once() {
    let Some(home) = bundled_python_home() else {
        return;
    };
    // SAFETY: called once before any other thread reads these env vars.
    unsafe {
        std::env::set_var("PYTHONHOME", &home);
        std::env::set_var("PYTHONNOUSERSITE", "1");
    }
}

pub fn bundled_python_home() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut candidates = Vec::new();
    if let Some(resources) = macos_app_resources(&exe) {
        let arch = std::env::consts::ARCH;
        candidates.push(resources.join(format!("python-{arch}")));
        candidates.push(resources.join("python"));
    }
    if let Some(dir) = exe.parent() {
        candidates.push(dir.join("python"));
        if let Some(parent) = dir.parent() {
            candidates.push(parent.join("python"));
        }
    }
    candidates.into_iter().find(|p| is_python_home(p))
}

fn macos_app_resources(exe: &Path) -> Option<PathBuf> {
    let mac_os = exe.parent()?;
    let contents = mac_os.parent()?;
    if contents.file_name().is_some_and(|n| n == "Contents") {
        Some(contents.join("Resources"))
    } else {
        None
    }
}

pub fn is_python_home(home: &Path) -> bool {
    let bin = home.join("bin").join("python3");
    if !(bin.is_file() || home.join("bin").join("python").is_file()) {
        return false;
    }
    home.join("lib")
        .read_dir()
        .ok()
        .into_iter()
        .flatten()
        .any(|e| {
            e.ok()
                .map(|e| e.file_name().to_string_lossy().starts_with("python3"))
                .unwrap_or(false)
        })
}

#[cfg(test)]
#[path = "tests/python_runtime__tests.rs"]
mod tests;
