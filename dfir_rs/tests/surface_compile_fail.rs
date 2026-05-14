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
    let source_dir = Path::new("tests/compile-fail");
    let stderr_dir = if cfg!(nightly) {
        Path::new("tests/compile-fail-nightly")
    } else {
        Path::new("tests/compile-fail-stable")
    };

    // Symlink all .rs files from the source directory into the channel-specific
    // stderr directory so trybuild can find them next to the .stderr files.
    for entry in fs::read_dir(source_dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let dest = stderr_dir.join(entry.file_name());
            let _ = fs::remove_file(&dest);
            let original = Path::new("..")
                .join(source_dir.file_name().unwrap())
                .join(entry.file_name());
            symlink(&original, &dest);
        }
    }

    let pattern = stderr_dir.join("surface_*.rs");
    let t = trybuild::TestCases::new();
    t.compile_fail(pattern.to_str().unwrap());
}
