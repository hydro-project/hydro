//! Shared locking and job directory utilities for parallel cargo builds in Hydro.
//!
//! This crate provides the coordination primitives for running multiple cargo
//! builds concurrently against a shared target directory using per-job symlinked
//! directories.
//!
//! # Why
//!
//! Hydro compiles each generated program (one per deployed process, sim test, ...)
//! as an example of a generated trybuild project. Cargo allows a single build per
//! target directory (it takes an exclusive lock on it), so compiling those programs
//! concurrently through cargo directly would serialize them. At the same time they
//! all share the same, expensive dependency graph, so giving each its own target
//! directory would recompile everything per program.
//!
//! # The trick: prebuild once, then share the artifacts without cargo's lock
//!
//! Builds run in two phases, coordinated by this crate's own locks:
//!
//! 1. **Prebuild** (exclusive `.prebuild.lock`): one cargo build compiles every
//!    dependency — including the trybuild dylib that the generated programs link
//!    against — into the *shared* target dir. While it runs, no final build is in
//!    flight, so it may freely write shared state.
//! 2. **Final builds** (shared `.prebuild.lock`): many cargo builds run
//!    concurrently, each with its *own* target-dir root under `jobs/<JOB>/`, so
//!    cargo's locks are per job and never contend. The shared, prebuilt state is
//!    made visible inside each job dir via symlinks. This is safe without cargo's
//!    lock only because of an invariant the two phases establish: **a final build
//!    reads shared state but never writes it** — everything it reads is fresh (the
//!    prebuild just built it, with identical features/rustflags, so cargo finds the
//!    fingerprints up to date), and everything it creates (its own example unit,
//!    its uplifted artifacts) has a job-unique path. Switching configuration
//!    (features, rustflags, toolchain) takes the lock exclusive again, draining all
//!    in-flight final builds before the prebuild rewrites shared state.
//!
//! Unrelated cargo processes building in the shared target dir (a developer's
//! `cargo build`, an IDE) are kept out the same way cargo keeps itself out: this
//! crate holds the shared target dir's own `.cargo-build-lock` **shared** for the
//! whole prebuild + final build, and cargo takes that file **exclusive** for any
//! build there, so the two mutually exclude ("Blocking waiting for file lock on
//! build directory"). This is also why *every* cargo spawned by this crate —
//! prebuild included — gets a `jobs/*` root rather than the shared dir: a child
//! cargo using the shared dir would wait forever on the lock its own parent holds.
//!
//! # Configurations and the `__CARGO_DEFAULT_LIB_METADATA` optimization
//!
//! A *configuration* is a (crate, features, rustflags, compiler) tuple. Cargo hashes
//! all of these into the filenames of rlibs, so artifacts of different
//! configurations coexist in the shared `deps/` and are cached side by side — with
//! one exception: the filename of a path package's **dylib** carries no hash
//! (`libX_dylib.so`), so every configuration would want to write the same file.
//! The trybuild dylib is thus the single shared resource configurations fight
//! over, and the design has two modes (see [`run_prebuild`], [`dylib_lib_name`]):
//!
//! - **Default**: the fingerprint of rustflags + compiler version
//!   ([`toolchain_fingerprint`]) is baked into the dylib's `[lib] name`, so
//!   different toolchains/rustflags get distinct files. Features are not part of
//!   the name, so switching features *replaces* the dylib: there is a single
//!   `.prebuild.lock`, every build holds it shared through its final build, and a
//!   configuration switch takes it exclusive — draining all in-flight builds
//!   before the prebuild renames the manifest and rebuilds. Concurrency is
//!   therefore limited to builds of **one configuration at a time**: while test
//!   binary A (features `{sim}`) has final builds in flight, test binary B
//!   (features `{sim, deploy}`) blocks until they finish, then rebuilds the dylib,
//!   then A blocks on B — alternating binaries ping-pong the shared dylib.
//! - **`__CARGO_DEFAULT_LIB_METADATA=1`** (set in CI): this undocumented cargo
//!   variable (used by rust-lang/rust's bootstrap) makes cargo hash dylib filenames
//!   too (`libX_dylib-<hash>.so`), so every configuration's dylib is a distinct
//!   file and the manifest never changes. Shared state becomes append-only across
//!   configurations — nothing one configuration writes is ever read by another —
//!   so the lock can be split per configuration: one `.prebuild-<key>.lock` each,
//!   with the cross-configuration `.global-prebuild.lock` taken only around a
//!   prebuild's actual `cargo build` so two prebuilds never write `deps/` at the
//!   same time. The concurrency this unlocks: **final builds of different
//!   configurations run simultaneously**, and a prebuild for one configuration
//!   runs while another configuration's final builds are in flight. For nextest,
//!   which runs many test binaries with differing feature sets at once, that is
//!   the difference between the binaries serializing on each other and all of them
//!   proceeding in parallel — and every configuration's dylib stays cached instead
//!   of being rebuilt on each switch.
//!
//! Why a prebuild for configuration B may overlap configuration A's final builds:
//! cargo only writes a unit's files when the unit is *dirty* (fresh units are not
//! touched at all; the legacy-layout `build-script-build` re-link rewrites only the
//! job-local uplift). B's prebuild therefore writes B-specific units, which have
//! B's hashes and are never read by A, plus any unit *shared* with A that is dirty.
//! A's final builds read only units A's own prebuild completed, so a shared unit is
//! fresh for B — unless its inputs changed since A's prebuild. That is the
//! precondition: all concurrent builds must come from one source state. Within a
//! test run it holds (test binaries are built before any runs, and the prebuild
//! stamp is keyed on the binary's mtime); running test processes from two
//! different source states at once is unsupported in either mode (in default mode
//! A's `--frozen` final builds would panic with "unexpected recompilation"). The
//! prebuild-vs-prebuild case is different — two configurations can both be dirty
//! on the same not-yet-built shared unit — which is exactly what
//! `.global-prebuild.lock` serializes.
//!
//! Mixing builds with and without the variable against one target dir is
//! unsupported: they would rename the shared manifest under different locks.
//! Because the hashed dylib name is what the final artifacts link against
//! (`DT_NEEDED`), loader search paths must include wherever that hashed file
//! lives; see [`dylib_search_dirs`].
//!
//! # Layouts
//!
//! Cargo's new build-dir layout (`-Zbuild-dir-new-layout`, default on current
//! nightly) moves everything under `build/<pkg>/<unit-hash>/`; see
//! [`BuildDirLayout`] (detected at compile time by `build.rs`). The same job-dir
//! structure works for both layouts: the job's `build/` is a real directory two
//! levels deep whose children are symlinks, so writes cargo makes at that level
//! (legacy: re-linking `build-script-build`; new: the job's own example unit, which
//! for the simulator is the *same* cargo unit in every job) stay job-local.
//!
//! ```text
//! LEGACY LAYOUT (stable)                                       NEW LAYOUT (nightly -Zbuild-dir-new-layout)
//!
//! target/                                                      target/
//! ├── .prebuild.lock       hydro's own lock (see above)        ├── .prebuild.lock
//! ├── debug/               <-- SHARED prebuilt state           ├── debug/               <-- SHARED prebuilt state
//! │   ├── .cargo-build-lock   hydro holds SHARED;              │   ├── .cargo-build-lock   (same: excludes unrelated cargo)
//! │   │                       unrelated cargo takes EXCLUSIVE  │   ├── (no .fingerprint/, no deps/)
//! │   ├── .fingerprint/                                        │   ├── build/
//! │   │   └── <unit>-<h>/     one dir per unit                 │   │   └── <pkg>/          one dir per package
//! │   ├── deps/               rlibs + dylibs, hashed names     │   │       └── <h>/        one dir per unit
//! │   │   ├── libfoo-<h>.rlib                                  │   │           ├── fingerprint/
//! │   │   └── libX_dylib-<h>.so   SHARED prebuilt dylib        │   │           ├── out/    rlibs, dylibs, .o
//! │   └── build/              build scripts + OUT_DIR          │   │           │   └── libX_dylib-<h>.so   SHARED prebuilt dylib
//! │       └── <pkg>-<h>/                                       │   │           └── run/    build script runs
//! │           ├── build-script-build                           │   └── libX_dylib.so       uplift, UNHASHED name
//! │           ├── out/                                         │
//! │           └── output                                       ├── jobs/
//! │                                                            │   ├── prebuild/           (hydro's exclusive lock held)
//! ├── jobs/                                                    │   │   └── debug/
//! │   ├── prebuild/           (hydro's exclusive lock held)    │   │       ├── .cargo-lock, .cargo-build-lock   (job-local)
//! │   │   └── debug/                                           │   │       ├── .fingerprint -> target/debug/.fingerprint  (unused)
//! │   │       ├── .cargo-lock                   (job-local)    │   │       ├── deps         -> target/debug/deps          (unused)
//! │   │       ├── .fingerprint -> target/debug/.fingerprint    │   │       └── build        -> target/debug/build    writes through
//! │   │       ├── deps         -> target/debug/deps            │   │
//! │   │       └── build        -> target/debug/build           │   └── <JOB>/              (one per generated program)
//! │   │                           (whole dir, writes through)  │       └── debug/
//! │   │                                                        │           ├── .cargo-lock, .cargo-build-lock   (job-local)
//! │   └── <JOB>/              (one per generated program)      │           ├── .fingerprint -> ...              (unused)
//! │       └── debug/                                           │           ├── deps         -> ...              (unused)
//! │           ├── .cargo-lock                   (job-local)    │           ├── build/                           REAL dir
//! │           ├── .fingerprint -> target/debug/.fingerprint    │           │   └── <pkg>/                       REAL dir
//! │           ├── deps         -> target/debug/deps            │           │       ├── <h1> -> target/debug/build/<pkg>/<h1>  prebuilt
//! │           ├── build/                        REAL dir       │           │       └── <h2>/                    REAL: this job's
//! │           │   └── <pkg>-<h>/                REAL dir       │           │           └── out/libsim_dylib-<h2>.so    own example
//! │           │       ├── build-script-build -> shared         │           ├── libX_dylib.so       job's uplift of the SHARED dylib
//! │           │       ├── out                -> shared         │           └── examples/<JOB>      FINAL artifact
//! │           │       └── output             -> shared         │
//! │           └── examples/<JOB>      FINAL artifact           │
//! ```
//!
//! `libX_dylib` is the *shared* trybuild dylib (the user crate + `hydro_lang` built
//! once by the prebuild) that every final artifact links against; `libsim_dylib` is
//! a *final* per-job artifact (the simulator's `cdylib`) that links against it.
//!
//! Loader search paths (rpath / `LD_LIBRARY_PATH`) for `libX_dylib` are computed by
//! [`dylib_search_dirs`] and only ever name state that outlives the build: the
//! job's `debug/` and the shared location (`deps/` on legacy, the shared unit `out/`
//! dirs on new) — never `jobs/<JOB>/debug/build/`, which is repopulated on every
//! build (and job dirs are shared by tests whose generated programs are identical).

use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

/// The filesystem layout cargo uses for intermediate build artifacts.
///
/// Detected at compile time by this crate's build script, which inspects the
/// shape of its own `OUT_DIR` (see `build.rs`). Use [`build_dir_layout`] to
/// query it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildDirLayout {
    /// The stable layout: `debug/{.fingerprint,deps,build,...}`, with final
    /// artifacts uplifted into `debug/` and dependency dylibs in `debug/deps/`.
    Legacy,
    /// The layout behind `-Zbuild-dir-new-layout` (default on current nightly):
    /// all intermediates (fingerprints, rlibs, build script outputs) live under
    /// `debug/build/<pkg>/<hash>/{fingerprint,out,run}/`, and final artifacts
    /// — including dependency dylibs — are uplifted into `debug/`.
    New,
}

/// The [`BuildDirLayout`] used by the toolchain that compiled this crate.
///
/// The runtime cargo invocations coordinated by this crate use the same
/// toolchain (rustup pins the toolchain for the whole process tree), so the
/// compile-time verdict matches the layout of the builds we coordinate;
/// switching toolchains recompiles this crate and re-runs the detection.
pub fn build_dir_layout() -> BuildDirLayout {
    if cfg!(new_build_dir_layout) {
        BuildDirLayout::New
    } else {
        BuildDirLayout::Legacy
    }
}

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
    let now = SystemTime::now()
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

/// Per-key in-process prebuild locks, mirroring the per-key lock files (see [`run_prebuild`]).
static DEP_BUILD_LOCKS_PER_KEY: LazyLock<Mutex<HashMap<String, &'static parking_lot::RwLock<()>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_dep_lock(key: &str) -> &'static parking_lot::RwLock<()> {
    let mut map = DEP_BUILD_LOCKS_PER_KEY.lock().unwrap();
    map.entry(key.to_owned())
        .or_insert_with(|| Box::leak(Box::new(parking_lot::RwLock::new(()))))
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

/// Guard holding the shared target dir's `.cargo-build-lock` **shared**.
///
/// Cargo takes this same file **exclusive** for the duration of any build in that target dir
/// (the "build directory" lock, cargo ≥ 1.98 and nightly), so holding it shared for the whole
/// prebuild + final build mutually excludes us from unrelated cargo processes building there
/// — which would otherwise race our symlinked writes into `deps/`, `.fingerprint/` and
/// `build/` without any lock between us. Concurrent hydro builds all hold it shared and so
/// proceed together.
///
/// Consequence: no cargo spawned while this is held may use the shared target dir as its
/// `--target-dir` — it would block forever on the lock its own parent holds. Every cargo we
/// spawn (prebuild included) therefore gets its own root under `jobs/`, with the shared
/// state reached through symlinks.
pub struct CargoBuildLock {
    _file: fs::File,
}

impl CargoBuildLock {
    pub fn lock_shared(lock_path: &Path) -> Self {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .unwrap();
        file.lock_shared().unwrap();
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
    /// Acquire an upgradable lock (in-process upgradable read + file shared). `key` selects
    /// the in-process lock and should identify `lock_path`.
    pub fn lock_upgradable(lock_path: &Path, key: &str) -> Self {
        let rw_guard = get_dep_lock(key).upgradable_read();
        let lock_dir = lock_path.parent().unwrap().to_owned();
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .unwrap();
        file.lock_shared().unwrap();
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
        let file = self._file;
        let rw_guard = match self._rw_guard {
            RwGuard::Upgradable(u) => parking_lot::RwLockUpgradableReadGuard::upgrade(u),
            _ => panic!("can only upgrade from upgradable"),
        };
        // Release per-key shared lock before acquiring global exclusive
        // to avoid deadlock (other processes hold per-key shared and wait for global).
        file.unlock().unwrap();
        let global_guard = GLOBAL_PREBUILD_LOCK.write();
        let global_file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.lock_dir.join(".global-prebuild.lock"))
            .unwrap();
        global_file.lock().unwrap();
        file.lock().unwrap();
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
///
/// The `.fingerprint` and `deps` symlinks are what shares prebuilt state under
/// the [`BuildDirLayout::Legacy`] layout. Under [`BuildDirLayout::New`] cargo
/// ignores both directories (everything lives under `build/`), so the symlinks
/// are harmless; they are created unconditionally because the layout is not
/// yet known when job dirs are set up (detection happens during the prebuild).
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
/// This prevents races between processes building the same job concurrently.
///
/// The shared `build/` is mirrored two levels deep: each top-level directory is
/// recreated job-locally and its *children* are symlinked to the shared copies.
/// Anything the final build reads is the prebuilt, immutable shared state;
/// anything it *creates or replaces* lands in the job-local directory instead
/// of racing other jobs. This is the same structure under both layouts:
///
/// - **Legacy** (`build/<pkg>-<hash>/{build-script-build,out,output,...}`):
///   cargo re-links `build-script-build` even on fresh builds; with per-child
///   symlinks that replaces the job-local link, not the shared file.
/// - **New** (`build/<pkg>/<unit-hash>/{fingerprint,out,run}`): the final
///   build's own example unit is created job-locally. This matters because
///   concurrent jobs can compile the *same* cargo unit — the simulator builds
///   the fixed `sim-dylib` example in every job (varied only via
///   `include!(env!(..))`, which affects freshness but not the unit hash) — so
///   sharing `build/` wholesale would have them write, and uplift, the same
///   `out/` files.
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

    // A stale symlink may be left over from an earlier version that shared
    // `build/` wholesale; `remove_dir_all` would follow it into the shared dir.
    if job_build.symlink_metadata().is_ok_and(|m| m.is_symlink()) {
        let _ = fs::remove_file(&job_build);
    } else {
        let _ = fs::remove_dir_all(&job_build);
    }
    fs::create_dir_all(&job_build).unwrap();
    if shared_build.exists() {
        for entry in fs::read_dir(&shared_build).unwrap() {
            let entry = entry.unwrap();
            if !entry.file_type().unwrap().is_dir() {
                continue;
            }
            let dest = job_build.join(entry.file_name());
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

/// The verbose version (`rustc -vV`) of the compiler child `cargo` invocations will use,
/// honoring the `RUSTC` override like cargo does.
static RUSTC_VERBOSE_VERSION: LazyLock<String> = LazyLock::new(|| {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = std::process::Command::new(rustc)
        .arg("-vV")
        .output()
        .expect("failed to run `rustc -vV`");
    assert!(output.status.success(), "`rustc -vV` failed");
    String::from_utf8(output.stdout).unwrap()
});

fn hash_hex(hash: impl FnOnce(&mut DefaultHasher)) -> String {
    let mut hasher = DefaultHasher::new();
    hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Fingerprint of the compilation *environment* a child build runs under: the rustflags (in
/// any stable encoding) and the compiler's verbose version.
///
/// Cargo keys the filenames of rlibs on both of these, so different fingerprints never
/// collide in the shared `deps/` and can be cached side by side. It strips that key from
/// dylib filenames unless `__CARGO_DEFAULT_LIB_METADATA` is set, so without that variable the
/// trybuild dylib bakes this fingerprint into its target name instead (see [`dylib_lib_name`]).
pub fn toolchain_fingerprint(rustflags: &str) -> String {
    hash_hex(|h| {
        rustflags.hash(h);
        RUSTC_VERBOSE_VERSION.hash(h);
    })
}

/// Whether cargo hashes the filenames of path-package dylibs. Cargo normally keeps those
/// stable (`libfoo.so`) so executables can embed them; the undocumented
/// `__CARGO_DEFAULT_LIB_METADATA` variable — which rust-lang/rust's bootstrap uses to build
/// libstd — forces the metadata hash onto them too, making every configuration's dylib a
/// distinct file.
fn lib_metadata_enabled() -> bool {
    std::env::var_os("__CARGO_DEFAULT_LIB_METADATA").is_some()
}

/// The `[lib] name` of the trybuild dylib package `package_name` when built under `rustflags`.
///
/// With `__CARGO_DEFAULT_LIB_METADATA` set this is cargo's default (the package name with
/// `-` replaced by `_`): cargo already hashes the filename per configuration, so the manifest
/// never needs to change. Without it the [`toolchain_fingerprint`] is appended so that
/// configurations get distinct `lib{name}.so` files and cache side by side instead of
/// clobbering one another. Features are not part of the name: they vary per build against one
/// shared manifest, and the prebuild lock already serializes feature switches.
pub fn dylib_lib_name(package_name: &str, rustflags: &str) -> String {
    let default = package_name.replace('-', "_");
    if lib_metadata_enabled() {
        default
    } else {
        format!("{default}_{}", toolchain_fingerprint(rustflags))
    }
}

/// Exclusive file lock on a trybuild project's `.hydro-trybuild-lock`, serializing all writes
/// to the project's generated files (manifests, sources) across processes. Released on drop.
pub fn lock_project(project_dir: &Path) -> fs::File {
    let file = fs::File::create(project_dir.join(".hydro-trybuild-lock")).unwrap();
    file.lock().unwrap();
    file
}

fn dylib_manifest_path(project_dir: &Path) -> PathBuf {
    project_dir.join("dylib").join("Cargo.toml")
}

fn read_manifest(path: &Path) -> Option<toml_edit::DocumentMut> {
    fs::read_to_string(path).ok()?.parse().ok()
}

fn manifest_lib_name(doc: &toml_edit::DocumentMut) -> Option<&str> {
    doc.get("lib")?.get("name")?.as_str()
}

/// Directories that a dynamically linked artifact built into the per-job artifact dir
/// `job_debug` (e.g. `jobs/<name>/debug`) needs on its library search path (rpath or
/// `LD_LIBRARY_PATH`) to load the trybuild dylib whose cargo package is `dylib_package_name`.
/// `shared_debug` is the shared target's `debug/` that the job mirrors.
///
/// Every returned path must stay valid for the artifact's whole lifetime, which outlives the
/// build (the binary is exec'd by the deployment later; the simulator `dlopen`s its cdylib after
/// the build returns). Job dirs are keyed by generated-source hash, so tests whose programs are
/// identical share one; a later build in the same job dir repopulates its `build/` (see
/// [`populate_job_build_dir`]), so nothing under `job_debug/build/` may be referenced. The
/// dylib's home is instead the shared prebuilt state, which only changes under the exclusive
/// prebuild lock.
///
/// Cargo uplifts the dylib into `job_debug` under its unhashed `lib<name>.so` filename, which
/// satisfies the loader whenever the artifact links against that name. With
/// `__CARGO_DEFAULT_LIB_METADATA` set, however, the artifact links against the hash-suffixed
/// `lib<name>-<hash>.so`:
///
/// - under the legacy layout every variant, hashed or not, lives in `deps/`, which
///   `job_debug/deps` symlinks to once at job setup;
/// - under the new layout `deps/` no longer exists and the uplift is *renamed* to the unhashed
///   name, so the hashed file only exists in the dylib unit's
///   `<shared_debug>/build/<dylib_package_name>/<hash>/out/` dir. Every such unit dir is
///   included; hashed names are unique per unit so this is unambiguous, and unhashed names
///   resolve from `job_debug` (listed first) before any unit dir.
pub fn dylib_search_dirs(
    dylib_package_name: &str,
    job_debug: &Path,
    shared_debug: &Path,
) -> Vec<PathBuf> {
    let mut dirs = vec![job_debug.to_owned()];
    match build_dir_layout() {
        BuildDirLayout::Legacy => dirs.push(job_debug.join("deps")),
        BuildDirLayout::New => {
            if let Ok(units) = fs::read_dir(shared_debug.join("build").join(dylib_package_name)) {
                let mut out_dirs: Vec<PathBuf> = units
                    .flatten()
                    .map(|unit| unit.path().join("out"))
                    .filter(|out| out.is_dir())
                    .collect();
                out_dirs.sort();
                dirs.extend(out_dirs);
            }
        }
    }
    dirs
}

/// The `[lib] name` currently in the dylib manifest of the trybuild project at `project_dir`,
/// if it has one. Call with [`lock_project`] held and write the manifest under the same lock.
///
/// Manifest generation must reuse this rather than compute a fresh name: the name only ever
/// changes under the exclusive prebuild lock (see [`run_prebuild`]), which is what guarantees
/// no in-flight final build links against a dylib named for another configuration.
pub fn current_dylib_lib_name(project_dir: &Path) -> Option<String> {
    let doc = read_manifest(&dylib_manifest_path(project_dir))?;
    manifest_lib_name(&doc).map(str::to_owned)
}

/// Point the project's dylib manifest at the [`dylib_lib_name`] for `rustflags`.
///
/// Dylib builds call this first thing in their [`run_prebuild`] `build_fn`, i.e. under the
/// exclusive prebuild lock, once no in-flight final build links the old name. It takes
/// [`lock_project`] so a concurrent manifest regeneration cannot read the old name and write it
/// back over the new one.
pub fn set_dylib_lib_name(project_dir: &Path, rustflags: &str) {
    let _project_lock = lock_project(project_dir);
    let manifest = dylib_manifest_path(project_dir);
    let mut doc = read_manifest(&manifest).expect("dylib manifest must exist before prebuild");
    let package_name = doc["package"]["name"].as_str().unwrap();
    let lib_name = dylib_lib_name(package_name, rustflags);
    if manifest_lib_name(&doc) != Some(&lib_name) {
        doc["lib"]["name"] = toml_edit::value(lib_name);
        fs::write(&manifest, doc.to_string()).unwrap();
    }
}

/// Run the prebuild phase with proper locking and freshness checking.
///
/// - `target_dir`: the shared target directory (e.g. `target/`)
/// - `crate_name`: unique identifier for the crate being built (included in hash)
/// - `features`: list of features for hashing
/// - `rustflags`: the rustflags the prebuild (and the final builds sharing its artifacts) are
///   compiled with; see [`toolchain_fingerprint`]
/// - `staged_paths`: paths to check mtime against for freshness
/// - `build_fn`: closure called with `prebuild_target` path under the exclusive lock; should
///   run cargo build(s), and for dylib projects first call [`set_dylib_lib_name`]
///
/// Returns `(PrebuildGuard, CargoBuildLock)` both held in shared mode.
/// Caller should keep both alive during the final build.
///
/// # Locking and freshness
///
/// Cargo itself supports one build per target dir; the per-job target dirs sidestep that, so
/// concurrent builds are safe only while everything they both write has a distinct filename.
/// Cargo hashes rustflags, compiler version and features into every filename except a path
/// package's dylib (unless `__CARGO_DEFAULT_LIB_METADATA` is set), so the dylib and the
/// manifest naming it are the one shared resource:
///
/// - Without `__CARGO_DEFAULT_LIB_METADATA` there is a single `.prebuild.lock`. Every build
///   holds it shared through its final build; switching configuration takes it exclusive,
///   which waits for all in-flight builds to finish before `build_fn` renames the manifest and
///   rebuilds the prebuild. A fresh stamp therefore implies the manifest names this
///   configuration's dylib.
/// - With it, cargo names every configuration's files distinctly and the manifest is constant,
///   so builds only need to serialize with others of the same `(crate, features, fingerprint)`
///   — one `.prebuild-<key>.lock` each — and configurations never wait on one another.
///
/// Running builds with and without the variable against one target dir is unsupported: they
/// would rename the shared manifest under different locks.
///
/// The freshness stamp stored in the lock file covers crate, features and fingerprint, so a
/// prebuild for a different configuration is never mistaken for fresh.
pub fn run_prebuild(
    target_dir: &Path,
    crate_name: &str,
    features: &[String],
    rustflags: &str,
    staged_paths: &[PathBuf],
    build_fn: impl FnOnce(&Path),
) -> (PrebuildGuard, CargoBuildLock) {
    // Acquire cargo build lock shared for the entire prebuild + final build duration.
    let shared_debug = target_dir.join("debug");
    fs::create_dir_all(&shared_debug).ok();
    let cargo_lock = CargoBuildLock::lock_shared(&shared_debug.join(".cargo-build-lock"));

    let toolchain = toolchain_fingerprint(rustflags);
    let fingerprint = hash_hex(|h| {
        crate_name.hash(h);
        let mut sorted = features.to_vec();
        sorted.sort();
        sorted.hash(h);
        toolchain.hash(h);
    });

    // Without `__CARGO_DEFAULT_LIB_METADATA` every configuration shares one fixed key, so this
    // lock doubles as the global one: a configuration switch (exclusive) drains every in-flight
    // build, not just those of the same configuration. `.global-prebuild.lock` (taken in
    // `upgrade`) only adds anything in the per-key case, where it keeps two configurations from
    // running their `cargo build`s into the shared `deps/` at the same time.
    let lock_key = if lib_metadata_enabled() {
        format!("prebuild-{fingerprint}")
    } else {
        "prebuild".to_owned()
    };
    let lock_path = target_dir.join(format!(".{lock_key}.lock"));

    let staged_mtime = staged_paths
        .iter()
        .filter_map(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH);

    // The stamp's freshness also covers the build-dir layout: layout changes
    // require a toolchain change, which also changes the mtime of
    // `current_exe`, which callers include in `staged_paths`.
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

    log_build_event(
        target_dir,
        &format!("prebuild: lock_upgradable, lock_key={lock_key}, fingerprint={fingerprint}"),
    );
    let guard = PrebuildGuard::lock_upgradable(&lock_path, &lock_key);
    if prebuild_is_fresh(&lock_path, &fingerprint) {
        log_build_event(target_dir, "prebuild: fresh, downgrading to shared");
        return (guard.downgrade(), cargo_lock);
    }

    log_build_event(target_dir, "prebuild: not fresh, upgrading to exclusive");
    let mut guard = guard.upgrade();
    log_build_event(target_dir, "prebuild: exclusive acquired");

    // Re-check after acquiring exclusive.
    if !prebuild_is_fresh(&lock_path, &fingerprint) {
        let shared_debug = target_dir.join("debug");
        let jobs_dir = target_dir.join("jobs");
        let prebuild_target = setup_job_dir(&jobs_dir, "prebuild", &shared_debug);
        symlink_prebuild_build_dir(&prebuild_target, &shared_debug);

        build_fn(&prebuild_target);

        // Write fingerprint:timestamp.
        use std::io::Seek;
        let now_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file = guard.file_mut();
        file.set_len(0).unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        write!(file, "{}:{}", fingerprint, now_nanos).unwrap();
    }

    (guard.downgrade(), cargo_lock)
}
