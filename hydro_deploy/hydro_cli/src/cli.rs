use std::fmt::Display;
use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::{AnyhowError, AnyhowWrapper};

#[derive(Parser, Debug)]
#[command(name = "Hydro Deploy", author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Deploys an application given a Python deployment script.
    Deploy {
        /// Path to the deployment script.
        config: PathBuf,
        /// Additional arguments to pass to the deployment script.
        #[arg(last(true))]
        args: Vec<String>,
    },
}

fn async_wrapper_module(py: Python<'_>) -> Result<Bound<'_, PyModule>, PyErr> {
    PyModule::from_code_bound(
        py,
        include_str!("../hydro/async_wrapper.py"),
        "wrapper.py",
        "wrapper",
    )
}

#[derive(Debug)]
struct PyErrWithTraceback {
    err_display: String,
    traceback: String,
}

impl std::error::Error for PyErrWithTraceback {}

impl Display for PyErrWithTraceback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.err_display)?;
        write!(f, "{}", self.traceback)
    }
}

fn deploy(config: PathBuf, args: Vec<String>) -> anyhow::Result<()> {
    Python::with_gil(|py| -> anyhow::Result<()> {
        let syspath = py.import_bound("sys")?.getattr("path")?;
        let syspath: &Bound<'_, PyList> = syspath
            .downcast::<PyList>()
            .map_err(|downcast_error| anyhow::Error::msg(downcast_error.to_string()))?;

        syspath.insert(0, PathBuf::from(".").canonicalize().unwrap())?;

        let filename = config.canonicalize().unwrap();
        let fun: Py<PyAny> = PyModule::from_code_bound(
            py,
            std::fs::read_to_string(config).unwrap().as_str(),
            filename.to_str().unwrap(),
            "",
        )
        .with_context(|| format!("failed to load deployment script: {}", filename.display()))?
        .getattr("main")
        .context("expected deployment script to define a `main` function, but one was not found")?
        .into();

        let wrapper = async_wrapper_module(py)?;
        match wrapper.call_method1("run", (fun, args)) {
            Ok(_) => Ok(()),
            Err(err) => {
                let traceback = err
                    .traceback_bound(py)
                    .context("traceback was expected but none found")
                    .and_then(|tb| Ok(tb.format()?))?
                    .trim()
                    .to_string();

                if err.is_instance_of::<AnyhowError>(py) {
                    let args = err
                        .value_bound(py)
                        .getattr("args")?
                        .extract::<Vec<AnyhowWrapper>>()?;
                    let wrapper = args.first().unwrap();
                    let underlying = &wrapper.underlying;
                    let mut underlying = underlying.blocking_write();
                    Err(underlying.take().unwrap()).context(traceback)
                } else {
                    Err(PyErrWithTraceback {
                        err_display: format!("{}", err),
                        traceback,
                    }
                    .into())
                }
            }
        }
    })?;

    Ok(())
}

#[pyfunction]
fn cli_entrypoint(args: Vec<String>) -> PyResult<()> {
    match Cli::try_parse_from(args) {
        Ok(args) => {
            let res = match args.command {
                Commands::Deploy { config, args } => deploy(config, args),
            };

            match res {
                Ok(_) => Ok(()),
                Err(err) => {
                    eprintln!("{:?}", err);
                    Err(PyErr::new::<PyException, _>(""))
                }
            }
        }
        Err(err) => {
            err.print().unwrap();
            Err(PyErr::new::<PyException, _>(""))
        }
    }
}

#[pymodule]
pub fn cli(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(cli_entrypoint, m)?)?;

    Ok(())
}
