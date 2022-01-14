use super::build::{PullBuild, PushBuild};
use super::connect::{PullConnect, PushConnect};
use super::surface::pivot::PivotSurface;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc::SyncSender;

use crate::compiled::pivot::Pivot;
use crate::scheduled::ctx::{InputPort, OutputPort};
use crate::scheduled::graph::Hydroflow;
use crate::scheduled::graph_ext::GraphExt;
use crate::scheduled::handoff::{CanReceive, Handoff, VecHandoff};
use crate::scheduled::input::Input;
use crate::scheduled::net::Message;

use super::surface::pull_handoff::HandoffPullSurface;
use super::surface::push_handoff::HandoffPushSurfaceReversed;
use super::surface::push_start::StartPushSurface;
use super::surface::{PullSurface, PushSurfaceReversed};

/// The user-facing entry point for the Surface API.
#[derive(Default)]
pub struct HydroflowBuilder {
    hydroflow: Hydroflow,
    port_connectors: Vec<Box<dyn PortConnector>>,
}
impl HydroflowBuilder {
    /// Creates a handoff, returning push and pull ends which can be chained
    /// using the Surface API.
    pub fn make_handoff<H, T>(
        &mut self,
    ) -> (HandoffPushSurfaceReversed<H, T>, HandoffPullSurface<H>)
    where
        H: Handoff + CanReceive<T>,
    {
        let (port_connector, push, pull) = BothPortConnector::with_neither();
        self.port_connectors.push(Box::new(port_connector));

        (push, pull)
    }

    /// Adds a `pivot` created via the Surface API.
    pub fn add_subgraph<Pull, Push>(&mut self, pivot: PivotSurface<Pull, Push>)
    where
        Pull: 'static + PullSurface,
        Push: 'static + PushSurfaceReversed<ItemIn = Pull::ItemOut>,
    {
        let ((pull_connect, push_connect), (mut pull_build, mut push_build)) = pivot.into_parts();

        let (input_ports, output_ports) = self
            .hydroflow
            .add_subgraph::<_, Pull::InputHandoffs, Push::OutputHandoffs>(
                move |_ctx, recv_ctx, send_ctx| {
                    let pull = pull_build.build(recv_ctx);
                    let push = push_build.build(send_ctx);
                    let pivot = Pivot::new(pull, push);
                    pivot.run();
                },
            );

        pull_connect.connect(input_ports);
        push_connect.connect(output_ports);
    }

    /// Creates a new external channel input.
    pub fn add_channel_input<T, W>(&mut self) -> (Input<T, SyncSender<T>>, HandoffPullSurface<W>)
    where
        T: 'static,
        W: 'static + Handoff + CanReceive<T>,
    {
        let (input, output_port) = self.hydroflow.add_channel_input();

        let (output_port_connector, pull) = BothPortConnector::with_output(output_port);
        self.port_connectors.push(Box::new(output_port_connector));

        (input, pull)
    }

    pub fn add_write_tcp_stream(
        &mut self,
        stream: tokio::net::TcpStream,
    ) -> HandoffPushSurfaceReversed<VecHandoff<Message>, Option<Message>> {
        let input_port = self.hydroflow.add_write_tcp_stream(stream);

        let (input_port_connector, push) =
            BothPortConnector::with_input::<Option<Message>>(input_port);
        self.port_connectors.push(Box::new(input_port_connector));

        push
    }

    pub fn add_read_tcp_stream(
        &mut self,
        stream: tokio::net::TcpStream,
    ) -> HandoffPullSurface<VecHandoff<Message>> {
        let output_port = self.hydroflow.add_read_tcp_stream(stream);

        let (output_port_connector, pull) = BothPortConnector::with_output(output_port);
        self.port_connectors.push(Box::new(output_port_connector));

        pull
    }

    #[allow(clippy::type_complexity)] // TODO(mingwei).
    pub fn add_tcp_stream(
        &mut self,
        stream: tokio::net::TcpStream,
    ) -> (
        HandoffPushSurfaceReversed<VecHandoff<Message>, Option<Message>>,
        HandoffPullSurface<VecHandoff<Message>>,
    ) {
        let (input_port, output_port) = self.hydroflow.add_tcp_stream(stream);

        let (input_port_connector, push) =
            BothPortConnector::with_input::<Option<Message>>(input_port);
        self.port_connectors.push(Box::new(input_port_connector));

        let (output_port_connector, pull) = BothPortConnector::with_output(output_port);
        self.port_connectors.push(Box::new(output_port_connector));

        (push, pull)
    }

    pub fn build(mut self) -> Hydroflow {
        for handoff_connector in self.port_connectors {
            handoff_connector.connect(&mut self.hydroflow);
        }
        self.hydroflow
    }

    /// Start a new branch for teeing.
    ///
    /// For example:
    /// ```ignore
    /// my_ints
    ///     .tee(
    ///         builder
    ///             .start_tee()
    ///             .filter(|&x| 0 == x % 2)
    ///             .for_each(|x| println!("Even: {}", x)),
    ///         builder
    ///             .start_tee()
    ///             .filter(|&x| 1 == x % 2)
    ///             .for_each(|x| println!("Odd: {}", x)));
    /// ```
    pub fn start_tee<T>(&self) -> StartPushSurface<T> {
        StartPushSurface::new()
    }
}

/// Internal helper struct, run to connect all handoffs on [`HydroflowBuilder::build()`].
trait PortConnector {
    // TODO(mingwei): return result for informative error message.
    fn connect(self: Box<Self>, hydroflow: &mut Hydroflow);
}

struct BothPortConnector<Hof>
where
    Hof: Handoff,
{
    output_port: Rc<Cell<Option<OutputPort<Hof>>>>,
    input_port: Rc<Cell<Option<InputPort<Hof>>>>,
}
impl<Hof> BothPortConnector<Hof>
where
    Hof: Handoff,
{
    pub fn with_output(output_port: OutputPort<Hof>) -> (Self, HandoffPullSurface<Hof>) {
        let this = Self {
            output_port: Rc::new(Cell::new(Some(output_port))),
            input_port: Default::default(),
        };
        let pull = HandoffPullSurface::new(Rc::clone(&this.input_port));
        (this, pull)
    }

    pub fn with_input<T>(input_port: InputPort<Hof>) -> (Self, HandoffPushSurfaceReversed<Hof, T>)
    where
        Hof: CanReceive<T>,
    {
        let this = Self {
            output_port: Default::default(),
            input_port: Rc::new(Cell::new(Some(input_port))),
        };
        let push = HandoffPushSurfaceReversed::new(Rc::clone(&this.output_port));
        (this, push)
    }

    pub fn with_neither<T>() -> (
        Self,
        HandoffPushSurfaceReversed<Hof, T>,
        HandoffPullSurface<Hof>,
    )
    where
        Hof: CanReceive<T>,
    {
        let this = Self {
            output_port: Default::default(),
            input_port: Default::default(),
        };
        let pull = HandoffPullSurface::new(Rc::clone(&this.input_port));
        let push = HandoffPushSurfaceReversed::new(Rc::clone(&this.output_port));

        (this, push, pull)
    }
}
impl<Hof> PortConnector for BothPortConnector<Hof>
where
    Hof: Handoff,
{
    fn connect(self: Box<Self>, hydroflow: &mut Hydroflow) {
        // TODO(mingwei): more informative error handling.
        if let (Some(output_port), Some(input_port)) =
            (self.output_port.take(), self.input_port.take())
        {
            hydroflow.add_edge(output_port, input_port);
        } else {
            panic!("Ports were never connected!!");
        }
    }
}
