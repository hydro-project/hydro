use std::ffi::OsStr;
use std::io::{Read, Write};
use std::process::{Child, Stdio};

#[doc(hidden)]
pub use ctor;

/// Environment variable used to signal that the current test binary should run the example's
/// `main` instead of the test harness. Set to `"1"` by [`ExampleChild::run_new`].
///
/// Examples may also check this variable to detect that they are running under an example test.
pub const RUN_MAIN_ENV_VAR: &str = "RUNNING_AS_EXAMPLE_TEST";

/// Runs the current example's `main` in a child process, returning an [`ExampleChild`] handle.
/// Only arguments need to be specified.
///
/// This must be invoked from within a `#[test]` inside an example (i.e. a file under
/// `examples/`), and the example must have a `main` function at the crate root.
///
/// Rather than invoking `cargo run --example ...` (which may trigger recompilation, as feature
/// flags often differ between `cargo test` and `cargo run`), this re-executes the
/// already-running test binary ([`std::env::current_exe`]). The example's `main` is already
/// compiled into the test binary, so no extra compilation is needed. A pre-main constructor
/// (registered via [`ctor`]) detects the re-execution (via the [`RUN_MAIN_ENV_VAR`] environment
/// variable) and calls `main` directly, before the test harness gets a chance to run. The
/// child process receives the example's arguments as its real `argv`, so `main` can parse
/// [`std::env::args`] normally (e.g. with `clap`).
#[macro_export]
#[expect(
    clippy::crate_in_macro_def,
    reason = "intentional: `crate::main` must refer to the *calling* example's `main`, not `example_test`"
)]
macro_rules! run_current_example {
    () => {
        $crate::run_current_example!(::std::iter::empty::<&str>())
    };
    ($args:literal) => {
        $crate::run_current_example!(str::split_whitespace($args))
    };
    ($args:expr $(,)?) => {{
        // Register a pre-main constructor (in the test binary) which, when the test binary is
        // re-executed by `ExampleChild::run_new`, runs the example's `main` and exits before the
        // test harness runs. When the env var is not set (i.e. in the normal test run), this is
        // a no-op. `priority = late` ensures this runs after other constructors (e.g. stageleft
        // rewrite registration in dependencies), since bin crate ctors otherwise run first.
        $crate::ctor::declarative::ctor! {
            #[ctor(unsafe, priority = late)]
            fn __example_test_run_main() {
                $crate::run_example_main_if_child(|| crate::main());
            }
        }
        $crate::ExampleChild::run_new($args)
    }};
}

/// Used by [`run_current_example!`]; do not call directly.
///
/// If [`RUN_MAIN_ENV_VAR`] is set, runs `main_fn` and exits the process with an appropriate exit
/// code. Otherwise does nothing.
pub fn run_example_main_if_child<T: ExampleMainReturn>(main_fn: impl FnOnce() -> T) {
    if std::env::var_os(RUN_MAIN_ENV_VAR).is_some_and(|value| value == "1") {
        let exit_code = main_fn().into_exit_code();
        std::process::exit(exit_code);
    }
}

/// Return types allowed for an example `main` run via [`run_current_example!`].
pub trait ExampleMainReturn {
    /// Converts the return value of `main` into a process exit code.
    fn into_exit_code(self) -> i32;
}
impl ExampleMainReturn for () {
    fn into_exit_code(self) -> i32 {
        0
    }
}
impl<T: ExampleMainReturn, E: std::fmt::Debug> ExampleMainReturn for Result<T, E> {
    fn into_exit_code(self) -> i32 {
        match self {
            Ok(inner) => inner.into_exit_code(),
            Err(err) => {
                eprintln!("Error: {:?}", err);
                1
            }
        }
    }
}

/// A wrapper around [`std::process::Child`] that allows us to wait for a specific outputs.
///
/// Terminates the inner [`Child`] process when dropped.
pub struct ExampleChild {
    child: Child,
    output_buffer: Vec<u8>,
    output_len: usize,
}
impl ExampleChild {
    pub fn run_new(args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        let current_exe = std::env::current_exe().expect("Failed to get current executable path.");
        let mut cmd = std::process::Command::new(current_exe);
        cmd.args(args).env(RUN_MAIN_ENV_VAR, "1");

        log::info!("Re-executing test binary as example: {:?}", cmd);

        let child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        Self {
            child,
            output_buffer: vec![0; 1024],
            output_len: 0,
        }
    }

    /// Waits for a specific string process output before returning.
    ///
    /// When a child process is spawned often you want to wait until the child process is ready before
    /// moving on. One way to do that synchronization is by waiting for the child process to output
    /// something and match regex against that output. For example, you could wait until the child
    /// process outputs "Client live!" which would indicate that it is ready to receive input now on
    /// stdin.
    pub fn read_string(&mut self, wait_for_string: &str) {
        self.read_regex(&regex::escape(wait_for_string));
    }

    /// Waits for a specific regex process output before returning.
    pub fn read_regex(&mut self, wait_for_regex: &str) {
        let stdout = self.child.stdout.as_mut().unwrap();
        let re = regex::Regex::new(wait_for_regex).unwrap();

        while !re.is_match(&String::from_utf8_lossy(
            &self.output_buffer[0..self.output_len],
        )) {
            eprintln!(
                "waiting ({}):\n{}",
                wait_for_regex,
                String::from_utf8_lossy(&self.output_buffer[0..self.output_len])
            );

            while self.output_buffer.len() - self.output_len < 1024 {
                self.output_buffer
                    .resize(self.output_buffer.len() + 1024, 0);
            }

            let bytes_read = stdout
                .read(&mut self.output_buffer[self.output_len..])
                .unwrap();
            self.output_len += bytes_read;

            if 0 == bytes_read {
                panic!("Child process exited before a match was found.");
            }
        }
    }

    /// Writes a line to the child process stdin. A newline is automatically appended and should not be included in `line`.
    pub fn write_line(&mut self, line: &str) {
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(line.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }
}

/// Terminates the inner [`Child`] process when dropped.
///
/// When a `Child` is dropped normally nothing happens but in unit tests you usually want to
/// terminate the child and wait for it to terminate. This does that for us.
impl Drop for ExampleChild {
    fn drop(&mut self) {
        #[cfg(target_family = "windows")]
        let _ = self.child.kill(); // Windows throws `PermissionDenied` if the process has already exited.
        #[cfg(not(target_family = "windows"))]
        self.child.kill().unwrap();

        self.child.wait().unwrap();
    }
}
