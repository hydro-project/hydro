use std::ffi::OsStr;
use std::io::{Read, Write};
use std::process::{Child, Stdio};

/// Calls [`ExampleChild::run_new`] with the package name and test name infered from the call site.
/// Only arguments need to be specified.
#[macro_export]
macro_rules! run_current_example {
    () => {
        $crate::run_current_example!(::std::iter::empty::<&str>())
    };
    ($args:literal) => {
        $crate::run_current_example!(str::split_whitespace($args))
    };
    ($args:expr $(,)?) => {
        $crate::ExampleChild::run_new(
            &::std::env::var("CARGO_PKG_NAME").unwrap(),
            &$crate::extract_example_name(file!()).expect("Failed to determine example name."),
            $args,
        )
    };
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
    pub fn run_new(
        pkg_name: &str,
        test_name: &str,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Self {
        let child = std::process::Command::new("cargo")
            .args(["run", "-p", pkg_name, "--example"])
            .arg(test_name)
            .arg("--")
            .args(args)
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

    /// Waits for a specific process output before returning.
    ///
    /// When a child process is spawned often you want to wait until the child process is ready before
    /// moving on. One way to do that synchronization is by waiting for the child process to output
    /// something and match regex against that output. For example, you could wait until the child
    /// process outputs "Client live!" which would indicate that it is ready to receive input now on
    /// stdin.
    pub fn wait_for_output(&mut self, wait_for_regex: &str) {
        let stdout = self.child.stdout.as_mut().unwrap();
        let re = regex::Regex::new(wait_for_regex).unwrap();

        while !re.is_match(&String::from_utf8_lossy(
            &self.output_buffer[0..self.output_len],
        )) {
            println!(
                "waiting ({}): {}",
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

/// Extract the example name from the [`std::file!`] path, used by the [`run_current_example!`] macro.
pub fn extract_example_name(file: &str) -> Option<String> {
    let pathbuf = std::path::PathBuf::from(file);
    let mut path = pathbuf.as_path();
    while path.parent()?.file_name()? != "examples" {
        path = path.parent().unwrap();
    }
    Some(path.file_stem()?.to_string_lossy().into_owned())
}
