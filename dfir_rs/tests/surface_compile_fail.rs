use std::fs;
use std::path::Path;

fn symlink(original: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(original, link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(original, link).unwrap();
}

#[test]
fn test_all() {
    let test_dir = Path::new("tests/compile-fail");
    let stderr_dir = test_dir.join(if cfg!(nightly) { "nightly" } else { "stable" });

    // Symlink all .rs files from the source directory into the channel-specific
    // stderr directory so trybuild can find them next to the .stderr files.
    for entry in fs::read_dir(test_dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let dest = stderr_dir.join(entry.file_name());
            let _ = fs::remove_file(&dest);
            let original = Path::new("..").join(entry.file_name());
            symlink(&original, &dest);
        }
    }

    let pattern = stderr_dir.join("surface_*.rs");
    let t = trybuild::TestCases::new();
    t.compile_fail(pattern.to_str().unwrap());
}
