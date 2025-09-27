use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::panic::RefUnwindSafe;

use dfir_rs::scheduled::graph::Dfir;
use libloading::Library;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc::UnboundedSender;

use crate::location::external_process::ExternalBincodeSink;

pub struct CompiledSim {
    pub(super) lib: Library,
    pub(super) external_ports: Vec<usize>,
}

pub trait Instantiator<'a>: RefUnwindSafe + Fn() -> CompiledSimInstance<'a> {}
impl<'a, T: RefUnwindSafe + Fn() -> CompiledSimInstance<'a>> Instantiator<'a> for T {}

impl CompiledSim {
    pub fn with_instance<T>(&self, thunk: impl FnOnce(CompiledSimInstance) -> T) -> T {
        let func: libloading::Symbol<
            unsafe extern "Rust" fn(HashMap<usize, Box<dyn Any>>) -> Dfir<'static>,
        > = unsafe { self.lib.get(b"__hydro_runtime").unwrap() };
        thunk(CompiledSimInstance {
            func,
            remaining_ports: self.external_ports.iter().cloned().collect(),
            instantiated_sources: HashMap::new(),
        })
    }

    pub fn with_instantiator<'a, T>(&'a self, thunk: impl FnOnce(&dyn Instantiator) -> T) -> T {
        let func: libloading::Symbol<
            unsafe extern "Rust" fn(HashMap<usize, Box<dyn Any>>) -> Dfir<'static>,
        > = unsafe { self.lib.get(b"__hydro_runtime").unwrap() };
        thunk(
            &(|| CompiledSimInstance {
                func: func.clone(),
                remaining_ports: self.external_ports.iter().cloned().collect(),
                instantiated_sources: HashMap::new(),
            }),
        )
    }
}

pub struct CompiledSimInstance<'a> {
    func: libloading::Symbol<
        'a,
        unsafe extern "Rust" fn(HashMap<usize, Box<dyn Any>>) -> Dfir<'static>,
    >,
    remaining_ports: HashSet<usize>,
    instantiated_sources: HashMap<usize, Box<dyn Any>>,
}

impl<'a> CompiledSimInstance<'a> {
    pub fn connect_sink_bincode<T: 'static + Send + Serialize + DeserializeOwned>(
        &mut self,
        port: &ExternalBincodeSink<T>,
    ) -> UnboundedSender<T> {
        assert!(self.remaining_ports.remove(&port.port_id));
        let (sink, source) = dfir_rs::util::unbounded_channel::<T>();
        self.instantiated_sources
            .insert(port.port_id, Box::new(source) as Box<dyn Any>);
        sink
    }

    pub fn dfir(self) -> Dfir<'static> {
        if !self.remaining_ports.is_empty() {
            panic!(
                "Cannot launch DFIR because some of the inputs / outputs have not been connected."
            )
        }

        let my_dfir = unsafe { (self.func)(self.instantiated_sources) };
        my_dfir
    }
}
