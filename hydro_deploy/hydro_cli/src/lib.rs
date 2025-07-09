#![expect(unsafe_op_in_unsafe_fn, reason = "for pyo3 generated code")]

use core::rust_crate::ports::RustCrateSource;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use bytes::Bytes;
use futures::{Future, SinkExt, StreamExt};
use hydro_deploy_integration::{
    ConnectedDirect, ConnectedSink, ConnectedSource, Connection, DynSink, DynStream,
};
use pyo3::exceptions::{PyException, PyStopAsyncIteration};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use pyo3::{create_exception, wrap_pymodule};
use pyo3_async_runtimes::TaskLocals;
use pythonize::pythonize;
use tokio::sync::oneshot::Sender;
use tokio::sync::{Mutex, RwLock};

mod cli;
use hydro_deploy::ssh::LaunchedSshHost;
use hydro_deploy::{self as core};

static TOKIO_RUNTIME: std::sync::RwLock<Option<tokio::runtime::Runtime>> =
    std::sync::RwLock::new(None);

#[pyfunction]
fn cleanup_runtime() {
    drop(TOKIO_RUNTIME.write().unwrap().take());
}

struct TokioRuntime {}

impl pyo3_async_runtimes::generic::Runtime for TokioRuntime {
    type JoinError = tokio::task::JoinError;
    type JoinHandle = tokio::task::JoinHandle<()>;

    fn spawn<F>(fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        TOKIO_RUNTIME
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .spawn(async move {
                fut.await;
            })
    }
}

tokio::task_local! {
    static TASK_LOCALS: OnceCell<TaskLocals>;
}

impl pyo3_async_runtimes::generic::ContextExt for TokioRuntime {
    fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
    where
        F: Future<Output = R> + Send + 'static,
    {
        let cell = OnceCell::new();
        cell.set(locals).unwrap();

        Box::pin(TASK_LOCALS.scope(cell, fut))
    }

    fn get_task_locals() -> Option<TaskLocals> {
        TASK_LOCALS
            .try_with(|c| c.get().cloned())
            .unwrap_or_default()
    }
}

create_exception!(hydro_cli_core, AnyhowError, PyException);

#[pyclass]
struct SafeCancelToken {
    cancel_tx: Option<Sender<()>>,
}

#[pymethods]
impl SafeCancelToken {
    fn safe_cancel(&mut self) {
        if let Some(token) = self.cancel_tx.take() {
            let _ = token.send(());
        }
    }
}

static CONVERTERS_MODULE: OnceLock<Py<PyModule>> = OnceLock::new();

fn interruptible_future_to_py<F, T>(py: Python<'_>, fut: F) -> PyResult<Bound<'_, PyAny>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject>,
{
    let module = CONVERTERS_MODULE.get().unwrap().clone().into_bound(py);

    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();

    let base_coro =
        pyo3_async_runtimes::generic::future_into_py::<TokioRuntime, _, _>(py, async move {
            tokio::select! {
                biased;
                _ = cancel_rx => Ok(None),
                r = fut => r.map(|o| Some(o))
            }
        })?;

    module.call_method1(
        "coroutine_to_safely_cancellable",
        (
            base_coro,
            SafeCancelToken {
                cancel_tx: Some(cancel_tx),
            },
        ),
    )
}

#[pyclass]
#[derive(Clone)]
pub struct AnyhowWrapper {
    pub underlying: Arc<RwLock<Option<anyhow::Error>>>,
}

#[pymethods]
impl AnyhowWrapper {
    fn __str__(&self) -> PyResult<String> {
        Ok(format!(
            "{:?}",
            self.underlying.try_read().unwrap().as_ref().unwrap()
        ))
    }
}

#[pyclass(subclass)]
#[derive(Clone)]
struct HydroflowSink {
    underlying: Arc<dyn core::rust_crate::ports::RustCrateSink>,
}

#[pyclass(name = "Deployment")]
pub struct Deployment {
    underlying: Arc<RwLock<core::Deployment>>,
}

#[pymethods]
impl Deployment {
    #[new]
    fn new() -> Self {
        Deployment {
            underlying: Arc::new(RwLock::new(core::Deployment::new())),
        }
    }

    #[expect(non_snake_case, reason = "pymethods")]
    fn Localhost(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let arc = self.underlying.blocking_read().Localhost();

        Ok(Py::new(
            py,
            PyClassInitializer::from(Host {
                underlying: arc.clone(),
            })
            .add_subclass(LocalhostHost { underlying: arc }),
        )?
        .into_py(py))
    }

    #[expect(non_snake_case, clippy::too_many_arguments, reason = "pymethods")]
    fn GcpComputeEngineHost(
        &self,
        py: Python<'_>,
        project: String,
        machine_type: String,
        image: String,
        region: String,
        network: GcpNetwork,
        user: Option<String>,
    ) -> PyResult<Py<PyAny>> {
        let arc = self.underlying.blocking_write().add_host(|id| {
            core::GcpComputeEngineHost::new(
                id,
                project,
                machine_type,
                image,
                region,
                network.underlying,
                user,
            )
        });

        Ok(Py::new(
            py,
            PyClassInitializer::from(Host {
                underlying: arc.clone(),
            })
            .add_subclass(GcpComputeEngineHost { underlying: arc }),
        )?
        .into_py(py))
    }

    #[expect(non_snake_case, clippy::too_many_arguments, reason = "pymethods")]
    fn AzureHost(
        &self,
        py: Python<'_>,
        project: String,
        os_type: String, // linux or windows
        machine_size: String,
        region: String,
        image: Option<HashMap<String, String>>,
        user: Option<String>,
    ) -> PyResult<Py<PyAny>> {
        let arc = self.underlying.blocking_write().add_host(|id| {
            core::AzureHost::new(id, project, os_type, machine_size, image, region, user)
        });

        Ok(Py::new(
            py,
            PyClassInitializer::from(Host {
                underlying: arc.clone(),
            })
            .add_subclass(AzureHost { underlying: arc }),
        )?
        .into_py(py))
    }

    #[expect(non_snake_case, reason = "pymethods")]
    fn CustomService(
        &self,
        py: Python<'_>,
        on: &Host,
        external_ports: Vec<u16>,
    ) -> PyResult<Py<PyAny>> {
        let service = self
            .underlying
            .blocking_write()
            .CustomService(on.underlying.clone(), external_ports);

        Ok(Py::new(
            py,
            PyClassInitializer::from(Service {
                underlying: service.clone(),
            })
            .add_subclass(CustomService {
                underlying: service,
            }),
        )?
        .into_py(py))
    }

    #[expect(non_snake_case, clippy::too_many_arguments, reason = "pymethods")]
    fn HydroflowCrate(
        &self,
        py: Python<'_>,
        src: String,
        on: &Host,
        bin: Option<String>,
        example: Option<String>,
        profile: Option<String>,
        features: Option<Vec<String>>,
        config: Option<String>,
        args: Option<Vec<String>>,
        display_id: Option<String>,
        external_ports: Option<Vec<u16>>,
    ) -> PyResult<Py<PyAny>> {
        let service = self.underlying.blocking_write().add_service(|id| {
            core::rust_crate::RustCrateService::new(
                id,
                src.into(),
                on.underlying.clone(),
                bin,
                example,
                profile,
                None,  // Python API doesn't support rustflags
                None,  // Python API doesn't support target_dir
                false, // Python API doesn't support no_default_features
                None,  // Python API doesn't support perf
                features,
                config,
                args,
                display_id,
                external_ports.unwrap_or_default(),
            )
        });

        Ok(Py::new(
            py,
            PyClassInitializer::from(Service {
                underlying: service.clone(),
            })
            .add_subclass(HydroflowCrate {
                underlying: service,
            }),
        )?
        .into_py(py))
    }

    fn deploy<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let underlying = self.underlying.clone();
        let py_none = py.None();
        interruptible_future_to_py(py, async move {
            underlying.write().await.deploy().await.map_err(|e| {
                AnyhowError::new_err(AnyhowWrapper {
                    underlying: Arc::new(RwLock::new(Some(e))),
                })
            })?;
            Ok(py_none)
        })
    }

    fn start<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let underlying = self.underlying.clone();
        let py_none = py.None();
        interruptible_future_to_py(py, async move {
            underlying.write().await.start().await.map_err(|e| {
                AnyhowError::new_err(AnyhowWrapper {
                    underlying: Arc::new(RwLock::new(Some(e))),
                })
            })?;
            Ok(py_none)
        })
    }
}

#[pyclass(subclass)]
pub struct Host {
    underlying: Arc<dyn core::Host>,
}

#[pyclass(extends=Host, subclass)]
struct LocalhostHost {
    underlying: Arc<core::LocalhostHost>,
}

#[pymethods]
impl LocalhostHost {
    fn client_only(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let arc = Arc::new(self.underlying.client_only());

        Ok(Py::new(
            py,
            PyClassInitializer::from(Host {
                underlying: arc.clone(),
            })
            .add_subclass(LocalhostHost { underlying: arc }),
        )?
        .into_py(py))
    }
}

#[pyclass]
#[derive(Clone)]
struct GcpNetwork {
    underlying: Arc<RwLock<core::gcp::GcpNetwork>>,
}

#[pymethods]
impl GcpNetwork {
    #[new]
    fn new(project: String, existing: Option<String>) -> Self {
        GcpNetwork {
            underlying: Arc::new(RwLock::new(core::gcp::GcpNetwork::new(project, existing))),
        }
    }
}

#[pyclass(extends=Host, subclass)]
struct GcpComputeEngineHost {
    underlying: Arc<core::GcpComputeEngineHost>,
}

#[pymethods]
impl GcpComputeEngineHost {
    #[getter]
    fn internal_ip(&self) -> String {
        self.underlying.launched.get().unwrap().internal_ip.clone()
    }

    #[getter]
    fn external_ip(&self) -> Option<String> {
        self.underlying.launched.get().unwrap().external_ip.clone()
    }

    #[getter]
    fn ssh_key_path(&self) -> String {
        self.underlying
            .launched
            .get()
            .unwrap()
            .ssh_key_path()
            .to_str()
            .unwrap()
            .to_string()
    }
}

#[pyclass(extends=Host, subclass)]
struct AzureHost {
    underlying: Arc<core::AzureHost>,
}

#[pymethods]
impl AzureHost {
    #[getter]
    fn internal_ip(&self) -> String {
        self.underlying.launched.get().unwrap().internal_ip.clone()
    }

    #[getter]
    fn external_ip(&self) -> Option<String> {
        self.underlying.launched.get().unwrap().external_ip.clone()
    }

    #[getter]
    fn ssh_key_path(&self) -> String {
        self.underlying
            .launched
            .get()
            .unwrap()
            .ssh_key_path()
            .to_str()
            .unwrap()
            .to_string()
    }
}

#[pyclass(subclass)]
pub struct Service {
    underlying: Arc<RwLock<dyn core::Service>>,
}

#[pymethods]
impl Service {
    fn stop<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let underlying = self.underlying.clone();
        let py_none = py.None();
        interruptible_future_to_py(py, async move {
            underlying.write().await.stop().await.unwrap();
            Ok(py_none)
        })
    }
}

#[pyclass]
struct PyReceiver {
    receiver: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<String>>>,
}

#[pymethods]
impl PyReceiver {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'p>(&self, py: Python<'p>) -> Option<Bound<'p, PyAny>> {
        let receiver = self.receiver.clone();
        Some(
            interruptible_future_to_py(py, async move {
                receiver
                    .lock()
                    .await
                    .recv()
                    .await
                    .ok_or_else(|| PyStopAsyncIteration::new_err(()))
            })
            .unwrap(),
        )
    }
}

#[pyclass(extends=Service, subclass)]
struct CustomService {
    underlying: Arc<RwLock<core::CustomService>>,
}

#[pymethods]
impl CustomService {
    fn client_port(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let arc = Arc::new(core::custom_service::CustomClientPort::new(Arc::downgrade(
            &self.underlying,
        )));

        Ok(Py::new(
            py,
            PyClassInitializer::from(HydroflowSink {
                underlying: arc.clone(),
            })
            .add_subclass(CustomClientPort { underlying: arc }),
        )?
        .into_py(py))
    }
}

#[pyclass(extends=HydroflowSink, subclass)]
#[derive(Clone)]
struct CustomClientPort {
    underlying: Arc<core::custom_service::CustomClientPort>,
}

#[pymethods]
impl CustomClientPort {
    fn send_to(&self, to: &HydroflowSink) {
        self.underlying.send_to(to.underlying.deref());
    }

    fn tagged(&self, tag: u32) -> TaggedSource {
        TaggedSource {
            underlying: Arc::new(core::rust_crate::ports::TaggedSource {
                source: self.underlying.clone(),
                tag,
            }),
        }
    }

    fn server_port<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let underlying = self.underlying.clone();
        interruptible_future_to_py(py, async move {
            Ok(ServerPort {
                underlying: underlying.server_port().await,
            })
        })
    }
}

#[pyclass(extends=Service, subclass)]
struct HydroflowCrate {
    underlying: Arc<RwLock<core::rust_crate::RustCrateService>>,
}

#[pymethods]
impl HydroflowCrate {
    fn stdout<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let underlying = self.underlying.clone();
        interruptible_future_to_py(py, async move {
            let underlying = underlying.read().await;
            Ok(PyReceiver {
                receiver: Arc::new(Mutex::new(underlying.stdout())),
            })
        })
    }

    fn stderr<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let underlying = self.underlying.clone();
        interruptible_future_to_py(py, async move {
            let underlying = underlying.read().await;
            Ok(PyReceiver {
                receiver: Arc::new(Mutex::new(underlying.stderr())),
            })
        })
    }

    fn exit_code<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let underlying = self.underlying.clone();
        interruptible_future_to_py(py, async move {
            let underlying = underlying.read().await;
            Ok(underlying.exit_code())
        })
    }

    #[getter]
    fn ports(&self) -> HydroflowCratePorts {
        HydroflowCratePorts {
            underlying: self.underlying.clone(),
        }
    }
}

#[pyclass]
#[derive(Clone)]
struct HydroflowCratePorts {
    underlying: Arc<RwLock<core::rust_crate::RustCrateService>>,
}

#[pymethods]
impl HydroflowCratePorts {
    fn __getattribute__(&self, name: String, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let arc = Arc::new(
            self.underlying
                .try_read()
                .unwrap()
                .get_port(name, &self.underlying),
        );

        Ok(Py::new(
            py,
            PyClassInitializer::from(HydroflowSink {
                underlying: arc.clone(),
            })
            .add_subclass(HydroflowCratePort { underlying: arc }),
        )?
        .into_py(py))
    }
}

#[pyclass(extends=HydroflowSink, subclass)]
#[derive(Clone)]
struct HydroflowCratePort {
    underlying: Arc<core::rust_crate::ports::RustCratePortConfig>,
}

#[pymethods]
impl HydroflowCratePort {
    fn merge(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let arc = Arc::new(self.underlying.clone().merge());

        Ok(Py::new(
            py,
            PyClassInitializer::from(HydroflowSink {
                underlying: arc.clone(),
            })
            .add_subclass(HydroflowCratePort { underlying: arc }),
        )?
        .into_py(py))
    }

    fn send_to(&self, to: &HydroflowSink) {
        self.underlying.send_to(to.underlying.deref());
    }

    fn tagged(&self, tag: u32) -> TaggedSource {
        TaggedSource {
            underlying: Arc::new(core::rust_crate::ports::TaggedSource {
                source: self.underlying.clone(),
                tag,
            }),
        }
    }
}

#[pyfunction]
fn demux(mapping: Bound<'_, PyDict>) -> HydroflowSink {
    HydroflowSink {
        underlying: Arc::new(core::rust_crate::ports::DemuxSink {
            demux: mapping
                .into_iter()
                .map(|(k, v)| {
                    let k = k.extract::<u32>().unwrap();
                    let v = v.extract::<HydroflowSink>().unwrap();
                    (k, v.underlying)
                })
                .collect(),
        }),
    }
}

#[pyclass(subclass)]
#[derive(Clone)]
struct TaggedSource {
    underlying: Arc<core::rust_crate::ports::TaggedSource>,
}

#[pymethods]
impl TaggedSource {
    fn send_to(&self, to: &HydroflowSink) {
        self.underlying.send_to(to.underlying.deref());
    }

    fn tagged(&self, tag: u32) -> TaggedSource {
        TaggedSource {
            underlying: Arc::new(core::rust_crate::ports::TaggedSource {
                source: self.underlying.clone(),
                tag,
            }),
        }
    }
}

#[pyclass(extends=HydroflowSink, subclass)]
#[derive(Clone)]
struct HydroflowNull {
    underlying: Arc<core::rust_crate::ports::NullSourceSink>,
}

#[pymethods]
impl HydroflowNull {
    fn send_to(&self, to: &HydroflowSink) {
        self.underlying.send_to(to.underlying.deref());
    }

    fn tagged(&self, tag: u32) -> TaggedSource {
        TaggedSource {
            underlying: Arc::new(core::rust_crate::ports::TaggedSource {
                source: self.underlying.clone(),
                tag,
            }),
        }
    }
}

#[pyfunction]
fn null(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let arc = Arc::new(core::rust_crate::ports::NullSourceSink);

    Ok(Py::new(
        py,
        PyClassInitializer::from(HydroflowSink {
            underlying: arc.clone(),
        })
        .add_subclass(HydroflowNull { underlying: arc }),
    )?
    .into_py(py))
}

#[pyclass]
struct ServerPort {
    underlying: hydro_deploy_integration::ServerPort,
}

fn with_tokio_runtime<T>(f: impl Fn() -> T) -> T {
    let runtime_read = TOKIO_RUNTIME.read().unwrap();
    let _guard = runtime_read.as_ref().unwrap().enter();
    f()
}

#[pymethods]
impl ServerPort {
    fn json(&self, py: Python<'_>) -> Py<PyAny> {
        pythonize(py, &self.underlying).unwrap()
    }

    #[expect(clippy::wrong_self_convention, reason = "pymethods")]
    fn into_source<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let realized = with_tokio_runtime(|| Connection::AsClient(self.underlying.connect()));

        interruptible_future_to_py(py, async move {
            Ok(PythonStream {
                underlying: Arc::new(RwLock::new(
                    realized.connect::<ConnectedDirect>().await.into_source(),
                )),
            })
        })
    }

    #[expect(clippy::wrong_self_convention, reason = "pymethods")]
    fn into_sink<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let realized = with_tokio_runtime(|| Connection::AsClient(self.underlying.connect()));

        interruptible_future_to_py(py, async move {
            Ok(PythonSink {
                underlying: Arc::new(RwLock::new(
                    realized.connect::<ConnectedDirect>().await.into_sink(),
                )),
            })
        })
    }
}

#[pyclass]
#[derive(Clone)]
struct PythonSink {
    underlying: Arc<RwLock<DynSink<Bytes>>>,
}

#[pymethods]
impl PythonSink {
    fn send<'p>(&self, data: Py<PyBytes>, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let underlying = self.underlying.clone();
        let bytes = Bytes::from(data.as_bytes(py).to_vec());
        interruptible_future_to_py(py, async move {
            underlying.write().await.send(bytes).await?;
            Ok(())
        })
    }
}

#[pyclass]
#[derive(Clone)]
struct PythonStream {
    underlying: Arc<RwLock<DynStream>>,
}

#[pymethods]
impl PythonStream {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'p>(&self, py: Python<'p>) -> Option<Bound<'p, PyAny>> {
        let underlying = self.underlying.clone();
        Some(
            interruptible_future_to_py(py, async move {
                let read_res = underlying.write().await.next().await;
                read_res
                    .and_then(|b| b.ok().map(|b| b.to_vec()))
                    .map(Ok)
                    .unwrap_or(Err(PyStopAsyncIteration::new_err(())))
            })
            .unwrap(),
        )
    }
}

#[pymodule]
pub fn _core(py: Python<'_>, module: Bound<'_, PyModule>) -> PyResult<()> {
    unsafe {
        pyo3::ffi::PyEval_InitThreads();
    }

    CONVERTERS_MODULE
        .set(
            PyModule::from_code_bound(
                py,
                "
import asyncio
async def coroutine_to_safely_cancellable(c, cancel_token):
    try:
        return await asyncio.shield(c)
    except asyncio.CancelledError:
        cancel_token.safe_cancel()
        await c
        raise asyncio.CancelledError()
",
                "converters",
                "converters",
            )?
            .into(),
        )
        .unwrap();

    *TOKIO_RUNTIME.write().unwrap() = Some(tokio::runtime::Runtime::new().unwrap());
    let atexit = PyModule::import_bound(py, "atexit")?;
    atexit.call_method1("register", (wrap_pyfunction!(cleanup_runtime, &module)?,))?;

    module.add("AnyhowError", py.get_type_bound::<AnyhowError>())?;
    module.add_class::<AnyhowWrapper>()?;

    module.add_class::<HydroflowSink>()?;
    module.add_class::<Deployment>()?;

    module.add_class::<Host>()?;
    module.add_class::<LocalhostHost>()?;

    module.add_class::<GcpNetwork>()?;
    module.add_class::<GcpComputeEngineHost>()?;

    module.add_class::<Service>()?;
    module.add_class::<CustomService>()?;
    module.add_class::<CustomClientPort>()?;
    module.add_class::<HydroflowCrate>()?;
    module.add_class::<HydroflowCratePort>()?;

    module.add_class::<ServerPort>()?;
    module.add_class::<PythonSink>()?;
    module.add_class::<PythonStream>()?;

    module.add_function(wrap_pyfunction!(demux, &module)?)?;
    module.add_function(wrap_pyfunction!(null, &module)?)?;

    module.add_wrapped(wrap_pymodule!(cli::cli))?;

    Ok(())
}
