//! Helper extensions for [`Dfir`].

use core::task;
use std::borrow::Cow;
use std::sync::mpsc::SyncSender;
use std::task::Poll;

use futures::Stream;

use super::SubgraphId;
use super::context::Context;
use super::graph::Dfir;
use super::handoff::{CanReceive, Handoff};
use super::input::Input;
use super::port::{RecvCtx, RecvPort, SendCtx, SendPort};

macro_rules! subgraph_ext {
    (
        $fn_name:ident,
        ( $($recv_param:ident : $recv_generic:ident),* ),
        ( $($send_param:ident : $send_generic:ident),* )
    ) => {
        /// Adds a subgraph with specific topology:
        ///
        #[doc = concat!("* Inputs: ", $( stringify!( $recv_generic ), ", ", )*)]
        #[doc = concat!("* Outputs: ", $( stringify!( $send_generic ), ", ", )*)]
        fn $fn_name <Name, F, $($recv_generic,)* $($send_generic),*> (
            &mut self,
            name: Name,
            $($recv_param : RecvPort< $recv_generic >,)*
            $($send_param : SendPort< $send_generic >,)* subgraph: F
        ) -> SubgraphId
        where
            Name: Into<Cow<'static, str>>,
            F: 'static + FnMut(&Context, $(&RecvCtx< $recv_generic >,)* $(&SendCtx< $send_generic >),*),
            $($recv_generic : 'static + Handoff,)*
            $($send_generic : 'static + Handoff,)*;
    };
    (
        impl
        $fn_name:ident,
        ( $($recv_param:ident : $recv_generic:ident),* ),
        ( $($send_param:ident : $send_generic:ident),* )
    ) => {
        fn $fn_name <Name, F, $($recv_generic,)* $($send_generic),*> (
            &mut self,
            name: Name,
            $($recv_param : RecvPort< $recv_generic >,)*
            $($send_param : SendPort< $send_generic >,)* subgraph: F
        ) -> SubgraphId
        where
            Name: Into<Cow<'static, str>>,
            F: 'static + FnMut(&Context, $(&RecvCtx< $recv_generic >,)* $(&SendCtx< $send_generic >),*),
            $($recv_generic : 'static + Handoff,)*
            $($send_generic : 'static + Handoff,)*
        {
            let mut subgraph = subgraph;
            self.add_subgraph(
                name,
                crate::var_expr!($($recv_param),*),
                crate::var_expr!($($send_param),*),
                move |ctx, crate::var_args!($($recv_param),*), crate::var_args!($($send_param),*)| {
                    (subgraph)(ctx, $($recv_param,)* $($send_param),*);
                    ::std::future::ready(())
                }
            )
        }
    };
}

/// Convenience extension methods for the [`Dfir`] struct.
///
/// Prefer DFIR syntax to avoid subgraph-per-op strucuring.
#[sealed::sealed]
pub trait GraphExt {
    subgraph_ext!(add_subgraph_sink, (recv_port: R), ());
    subgraph_ext!(add_subgraph_2sink, (recv_port_1: R1, recv_port_2: R2), ());

    subgraph_ext!(add_subgraph_source, (), (send_port: W));

    subgraph_ext!(add_subgraph_in_out, (recv_port: R), (send_port: W));
    subgraph_ext!(
        add_subgraph_in_2out,
        (recv_port: R),
        (send_port_1: W1, send_port_2: W2)
    );

    subgraph_ext!(
        add_subgraph_2in_out,
        (recv_port_1: R1, recv_port_2: R2),
        (send_port: W)
    );
    subgraph_ext!(
        add_subgraph_2in_2out,
        (recv_port_1: R1, recv_port_2: R2),
        (send_port_1: W1, send_port_2: W2)
    );

    /// Adds a channel input which sends to the `send_port`.
    fn add_channel_input<Name, T, W>(
        &mut self,
        name: Name,
        send_port: SendPort<W>,
    ) -> Input<T, SyncSender<T>>
    where
        Name: Into<Cow<'static, str>>,
        T: 'static,
        W: 'static + Handoff + CanReceive<T>;

    /// Adds an "input" operator, returning a handle to insert data into it.
    /// TODO(justin): make this thing work better
    fn add_input<Name, T, W>(
        &mut self,
        name: Name,
        send_port: SendPort<W>,
    ) -> Input<T, super::input::Buffer<T>>
    where
        Name: Into<Cow<'static, str>>,
        T: 'static,
        W: 'static + Handoff + CanReceive<T>;

    /// Adds a subgraph which pulls from the async stream and sends to the `send_port`.
    fn add_input_from_stream<Name, T, W, S>(
        &mut self,
        name: Name,
        send_port: SendPort<W>,
        stream: S,
    ) where
        Name: Into<Cow<'static, str>>,
        S: 'static + Stream<Item = T>,
        W: 'static + Handoff + CanReceive<T>;
}

#[sealed::sealed]
impl GraphExt for Dfir<'_> {
    subgraph_ext!(impl add_subgraph_sink, (recv_port: R), ());
    subgraph_ext!(
        impl add_subgraph_2sink,
        (recv_port_1: R1, recv_port_2: R2),
        ()
    );

    subgraph_ext!(impl add_subgraph_source, (), (send_port: W));

    subgraph_ext!(impl add_subgraph_in_out, (recv_port: R), (send_port: W));
    subgraph_ext!(
        impl add_subgraph_in_2out,
        (recv_port: R),
        (send_port_1: W1, send_port_2: W2)
    );

    subgraph_ext!(
        impl add_subgraph_2in_out,
        (recv_port_1: R1, recv_port_2: R2),
        (send_port: W)
    );
    subgraph_ext!(
        impl add_subgraph_2in_2out,
        (recv_port_1: R1, recv_port_2: R2),
        (send_port_1: W1, send_port_2: W2)
    );

    fn add_channel_input<Name, T, W>(
        &mut self,
        name: Name,
        send_port: SendPort<W>,
    ) -> Input<T, SyncSender<T>>
    where
        Name: Into<Cow<'static, str>>,
        T: 'static,
        W: 'static + Handoff + CanReceive<T>,
    {
        use std::sync::mpsc;

        let (sender, receiver) = mpsc::sync_channel(8000);
        let sg_id = self.add_subgraph_source::<_, _, W>(name, send_port, move |_ctx, send| {
            for x in receiver.try_iter() {
                send.give(x);
            }
        });
        Input::new(self.reactor(), sg_id, sender)
    }

    fn add_input<Name, T, W>(
        &mut self,
        name: Name,
        send_port: SendPort<W>,
    ) -> Input<T, super::input::Buffer<T>>
    where
        Name: Into<Cow<'static, str>>,
        T: 'static,
        W: 'static + Handoff + CanReceive<T>,
    {
        let input = super::input::Buffer::default();
        let inner_input = input.clone();
        let sg_id = self.add_subgraph_source::<_, _, W>(name, send_port, move |_ctx, send| {
            for x in (*inner_input.0).borrow_mut().drain(..) {
                send.give(x);
            }
        });
        Input::new(self.reactor(), sg_id, input)
    }

    fn add_input_from_stream<Name, T, W, S>(
        &mut self,
        name: Name,
        send_port: SendPort<W>,
        stream: S,
    ) where
        Name: Into<Cow<'static, str>>,
        S: 'static + Stream<Item = T>,
        W: 'static + Handoff + CanReceive<T>,
    {
        let mut stream = Box::pin(stream);
        self.add_subgraph_source::<_, _, W>(name, send_port, move |ctx, send| {
            let waker = ctx.waker();
            let mut cx = task::Context::from_waker(&waker);
            while let Poll::Ready(Some(v)) = stream.as_mut().poll_next(&mut cx) {
                send.give(v);
            }
        });
    }
}
