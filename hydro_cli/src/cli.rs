use std::fmt::Display;
use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::{AnyhowError, AnyhowWrapper};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// deploys
    Deploy {
        config: PathBuf,
        #[arg(last(true))]
        args: Vec<String>,
    },
}

fn async_wrapper_module(py: Python) -> Result<&PyModule, PyErr> {
    PyModule::from_code(
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
        let syspath: &PyList = py
            .import("sys")
            .and_then(|s| s.getattr("path"))
            .and_then(|p| Ok(p.downcast::<PyList>()?))?;

        syspath.insert(0, PathBuf::from(".").canonicalize().unwrap())?;

        let filename = config.canonicalize().unwrap();
        let fun: Py<PyAny> = PyModule::from_code(
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
                    .traceback(py)
                    .context("traceback was expected but none found")
                    .and_then(|tb| Ok(tb.format()?))?
                    .trim()
                    .to_string();

                if err.is_instance_of::<AnyhowError>(py) {
                    let args = err
                        .value(py)
                        .getattr("args")?
                        .extract::<Vec<AnyhowWrapper>>()?;
                    let wrapper = args.get(0).unwrap();
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
pub fn cli(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(cli_entrypoint, m)?)?;

    Ok(())
}
