//! Shared locking and job directory utilities for parallel cargo builds in Hydro.
//!
//! This crate provides the coordination primitives for running multiple cargo
//! builds concurrently against a shared target directory using per-job symlinked
//! directories.

use std::collections::HashMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

/// Log a timestamped message to the build coordination log.
pub fn log_build_event(project_dir: &Path, msg: &str) {
    let log_path = project_dir.join("build-coordination.log");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let thread_id = std::thread::current().id();
    let _ = writeln!(file, "[{now}ms] [{thread_id:?}] {msg}");
}

// ---------------------------------------------------------------------------
// In-process RwLocks
// ---------------------------------------------------------------------------

/// Global prebuild serialization lock (in-process).
static GLOBAL_PREBUILD_LOCK: parking_lot::RwLock<()> = parking_lot::RwLock::new(());

/// Per-feature-hash locks for when `__CARGO_DEFAULT_LIB_METADATA` is set.
static DEP_BUILD_LOCK: parking_lot::RwLock<()> = parking_lot::RwLock::new(());
static DEP_BUILD_LOCKS_PER_HASH: LazyLock<
    Mutex<HashMap<String, &'static parking_lot::RwLock<()>>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_dep_lock(features_hash: Option<&str>) -> &'static parking_lot::RwLock<()> {
    match features_hash {
        None => &DEP_BUILD_LOCK,
        Some(hash) => {
            let mut map = DEP_BUILD_LOCKS_PER_HASH.lock().unwrap();
            map.entry(hash.to_owned())
                .or_insert_with(|| Box::leak(Box::new(parking_lot::RwLock::new(()))))
        }
    }
}

/// Per-job-dir mutexes for serializing job dir setup/population within a process.
static JOB_DIR_LOCKS: LazyLock<Mutex<HashMap<PathBuf, &'static Mutex<()>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_job_dir_lock(job_dir: &Path) -> &'static Mutex<()> {
    let mut map = JOB_DIR_LOCKS.lock().unwrap();
    map.entry(job_dir.to_owned())
        .or_insert_with(|| Box::leak(Box::new(Mutex::new(()))))
}

// ---------------------------------------------------------------------------
// CargoBuildLock
// ---------------------------------------------------------------------------

/// Guard holding the cargo build directory file lock (shared).
pub struct CargoBuildLock {
    _file: fs::File,
}

impl CargoBuildLock {
    pub fn lock_shared(lock_path: &Path) -> Self {
        eprintln!(
            "[hydro-build] acquiring .cargo-build-lock shared: {}",
            lock_path.display()
        );
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .unwrap();
        file.lock_shared().unwrap();
        eprintln!("[hydro-build] acquired .cargo-build-lock shared");
        CargoBuildLock { _file: file }
    }
}

// ---------------------------------------------------------------------------
// PrebuildGuard
// ---------------------------------------------------------------------------

/// Guard holding both in-process and file locks for prebuild coordination.
pub struct PrebuildGuard {
    _rw_guard: RwGuard,
    _global_guard: Option<parking_lot::RwLockWriteGuard<'static, ()>>,
    _global_file: Option<fs::File>,
    _file: fs::File,
    lock_dir: PathBuf,
}

#[expect(
    dead_code,
    reason = "variants hold lock guards that are released on drop"
)]
enum RwGuard {
    Read(parking_lot::RwLockReadGuard<'static, ()>),
    Upgradable(parking_lot::RwLockUpgradableReadGuard<'static, ()>),
    Write(parking_lot::RwLockWriteGuard<'static, ()>),
}

impl PrebuildGuard {
    /// Acquire an upgradable lock (in-process upgradable read + file shared).
    /// When `features_hash` is Some, uses a per-hash lock (for `__CARGO_DEFAULT_LIB_METADATA` mode).
    pub fn lock_upgradable(lock_path: &Path, features_hash: Option<&str>) -> Self {
        eprintln!(
            "[hydro-build] acquiring prebuild upgradable lock: {}",
            lock_path.display()
        );
        let rw_guard = get_dep_lock(features_hash).upgradable_read();
        let lock_dir = lock_path.parent().unwrap().to_owned();
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .unwrap();
        file.lock_shared().unwrap();
        eprintln!("[hydro-build] acquired prebuild upgradable lock");
        PrebuildGuard {
            _rw_guard: RwGuard::Upgradable(rw_guard),
            _global_guard: None,
            _global_file: None,
            _file: file,
            lock_dir,
        }
    }

    /// Upgrade to exclusive (in-process write + file exclusive + global locks).
    pub fn upgrade(self) -> Self {
        eprintln!("[hydro-build] upgrading prebuild lock to exclusive");
        let file = self._file;
        let rw_guard = match self._rw_guard {
            RwGuard::Upgradable(u) => parking_lot::RwLockUpgradableReadGuard::upgrade(u),
            _ => panic!("can only upgrade from upgradable"),
        };
        // Release per-feature shared lock before acquiring global exclusive
        // to avoid deadlock (other processes hold per-feature shared and wait for global).
        file.unlock().unwrap();
        eprintln!("[hydro-build] acquiring global prebuild file lock");
        let global_guard = GLOBAL_PREBUILD_LOCK.write();
        let global_file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.lock_dir.join(".global-prebuild.lock"))
            .unwrap();
        global_file.lock().unwrap();
        eprintln!("[hydro-build] acquiring exclusive on per-feature lock");
        file.lock().unwrap();
        eprintln!("[hydro-build] acquired prebuild exclusive lock");
        PrebuildGuard {
            _rw_guard: RwGuard::Write(rw_guard),
            _global_guard: Some(global_guard),
            _global_file: Some(global_file),
            _file: file,
            lock_dir: self.lock_dir,
        }
    }

    /// Downgrade from exclusive to shared (releases global lock).
    pub fn downgrade(self) -> Self {
        let file = self._file;
        file.lock_shared().unwrap();
        let rw_guard = match self._rw_guard {
            RwGuard::Write(w) => RwGuard::Read(parking_lot::RwLockWriteGuard::downgrade(w)),
            RwGuard::Upgradable(u) => {
                RwGuard::Read(parking_lot::RwLockUpgradableReadGuard::downgrade(u))
            }
            r @ RwGuard::Read(_) => r,
        };
        PrebuildGuard {
            _rw_guard: rw_guard,
            _global_guard: None,
            _global_file: None,
            _file: file,
            lock_dir: self.lock_dir,
        }
    }

    /// Get mutable access to the underlying file (for writing timestamps).
    pub fn file_mut(&mut self) -> &mut fs::File {
        &mut self._file
    }
}

// ---------------------------------------------------------------------------
// Job directory setup
// ---------------------------------------------------------------------------

/// Set up a job directory with symlinked .fingerprint and deps subdirs.
/// Does NOT set up build/ — use `populate_job_build_dir` for final builds
/// or manually symlink for prebuild.
pub fn setup_job_dir(jobs_dir: &Path, name: &str, shared_debug: &Path) -> PathBuf {
    let job = jobs_dir.join(name);
    let _lock = get_job_dir_lock(&job);
    let _lock = _lock.lock().unwrap();

    // File lock for cross-process serialization on the same job dir.
    fs::create_dir_all(&job).unwrap();
    let job_lock_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(job.join(".job.lock"))
        .unwrap();
    job_lock_file.lock().unwrap();

    let job_debug = job.join("debug");
    fs::create_dir_all(&job_debug).unwrap();
    for subdir in [".fingerprint", "deps"] {
        let link = job_debug.join(subdir);
        if !link.exists() {
            let target = shared_debug.join(subdir);
            fs::create_dir_all(&target).unwrap();
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &link).unwrap();
            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(&target, &link).unwrap();
        }
    }
    job
}

/// Symlink the build/ directory for prebuild jobs (writes through to shared).
pub fn symlink_prebuild_build_dir(prebuild_target: &Path, shared_debug: &Path) {
    let link = prebuild_target.join("debug").join("build");
    if !link.exists() {
        let target = shared_debug.join("build");
        fs::create_dir_all(&target).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&target, &link).unwrap();
    }
}

/// Guard that holds the job directory lock for the duration of a build.
pub struct JobBuildGuard {
    _mutex_guard: std::sync::MutexGuard<'static, ()>,
    _file: fs::File,
}

/// Populate the per-job build/ directory from the shared build/ directory.
/// Returns a guard that holds the job lock — keep alive for the entire final build.
/// This prevents races from cargo's `link_or_copy` on build script binaries.
pub fn populate_job_build_dir(job_debug: &Path, shared_debug: &Path) -> JobBuildGuard {
    let shared_build = shared_debug.join("build");
    let job_build = job_debug.join("build");

    let job_dir = job_debug.parent().unwrap();
    let mutex_guard = get_job_dir_lock(job_dir);
    let mutex_guard = mutex_guard.lock().unwrap();

    // Also hold a file lock for cross-process serialization on the same job dir.
    let job_lock_path = job_dir.join(".job.lock");
    let job_lock_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&job_lock_path)
        .unwrap();
    job_lock_file.lock().unwrap();

    let _ = fs::remove_dir_all(&job_build);
    fs::create_dir_all(&job_build).unwrap();
    if shared_build.exists() {
        for entry in fs::read_dir(&shared_build).unwrap() {
            let entry = entry.unwrap();
            if !entry.file_type().unwrap().is_dir() {
                continue;
            }
            let dest = job_build.join(entry.file_name());
            // Create dir and symlink each child individually so cargo can
            // safely remove+relink build-script-build without racing other jobs.
            fs::create_dir_all(&dest).unwrap();
            for file in fs::read_dir(entry.path()).unwrap() {
                let file = file.unwrap();
                let file_dest = dest.join(file.file_name());
                #[cfg(unix)]
                std::os::unix::fs::symlink(file.path(), &file_dest).unwrap();
                #[cfg(windows)]
                {
                    if file.file_type().unwrap().is_dir() {
                        std::os::windows::fs::symlink_dir(file.path(), &file_dest).unwrap();
                    } else {
                        std::os::windows::fs::symlink_file(file.path(), &file_dest).unwrap();
                    }
                }
            }
        }
    }

    JobBuildGuard {
        _mutex_guard: mutex_guard,
        _file: job_lock_file,
    }
}

// ---------------------------------------------------------------------------
// Prebuild orchestration
// ---------------------------------------------------------------------------

/// Run the prebuild phase with proper locking and freshness checking.
///
/// - `target_dir`: the shared target directory (e.g. `target/`)
/// - `crate_name`: unique identifier for the crate being built (included in hash)
/// - `features`: list of features for hashing
/// - `staged_paths`: paths to check mtime against for freshness
/// - `build_fn`: closure called with `prebuild_target` path; should run cargo build(s)
///
/// Returns `(PrebuildGuard, CargoBuildLock)` both held in shared mode.
/// Caller should keep both alive during the final build.
pub fn run_prebuild(
    target_dir: &Path,
    crate_name: &str,
    features: &[String],
    staged_paths: &[PathBuf],
    build_fn: impl FnOnce(&Path),
) -> (PrebuildGuard, CargoBuildLock) {
    // Acquire cargo build lock shared for the entire prebuild + final build duration.
    let shared_debug = target_dir.join("debug");
    fs::create_dir_all(&shared_debug).ok();
    let cargo_lock = CargoBuildLock::lock_shared(&shared_debug.join(".cargo-build-lock"));

    let features_hash = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        crate_name.hash(&mut hasher);
        let mut sorted = features.to_vec();
        sorted.sort();
        sorted.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    };

    let has_lib_metadata = std::env::var("__CARGO_DEFAULT_LIB_METADATA").is_ok();
    let lock_path = if has_lib_metadata {
        target_dir.join(format!(".prebuild-{features_hash}.lock"))
    } else {
        target_dir.join(".prebuild.lock")
    };

    let staged_mtime = staged_paths
        .iter()
        .filter_map(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let prebuild_is_fresh = |path: &Path, expected_hash: &str| -> bool {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| {
                let mut parts = s.trim().splitn(2, ':');
                let hash = parts.next()?;
                let nanos = parts.next()?.parse::<u128>().ok()?;
                let ts = SystemTime::UNIX_EPOCH
                    .checked_add(std::time::Duration::from_nanos(nanos as u64))?;
                Some((hash.to_owned(), ts))
            })
            .is_some_and(|(hash, ts)| hash == expected_hash && ts >= staged_mtime)
    };

    let lock_hash = if has_lib_metadata {
        Some(features_hash.as_str())
    } else {
        None
    };

    log_build_event(
        target_dir,
        &format!("prebuild: lock_upgradable, hash={features_hash}"),
    );
    let guard = PrebuildGuard::lock_upgradable(&lock_path, lock_hash);
    if prebuild_is_fresh(&lock_path, &features_hash) {
        log_build_event(target_dir, "prebuild: fresh, downgrading to shared");
        return (guard.downgrade(), cargo_lock);
    }

    log_build_event(target_dir, "prebuild: not fresh, upgrading to exclusive");
    let mut guard = guard.upgrade();
    log_build_event(target_dir, "prebuild: exclusive acquired");

    // Re-check after acquiring exclusive.
    if !prebuild_is_fresh(&lock_path, &features_hash) {
        let shared_debug = target_dir.join("debug");
        let jobs_dir = target_dir.join("jobs");
        let prebuild_target = setup_job_dir(&jobs_dir, "prebuild", &shared_debug);
        symlink_prebuild_build_dir(&prebuild_target, &shared_debug);

        build_fn(&prebuild_target);

        // Write features_hash:timestamp.
        use std::io::Seek;
        let now_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file = guard.file_mut();
        file.set_len(0).unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        write!(file, "{}:{}", features_hash, now_nanos).unwrap();
    }

    (guard.downgrade(), cargo_lock)
}
