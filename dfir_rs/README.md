# DFIR

DFIR (Dataflow Intermediate Representation) is a small dataflow compiler for creating low-latency single-node dataflow
programs in Rust. DFIR exposes both a macro interface which uses a custom domain-specific language known as the
_surface syntax_, as well as a graph-builder API. DFIR primarily serves as the lowest-level of the
[Hydro framework](https://hydro.run/docs/hydro) stack, turning Hydro graphs into compilable Rust code.

DFIR is targeted at supporting the following unique features:
1. Extremely low-latency and high-throughput via [Rust monomorphization](https://rustc-dev-guide.rust-lang.org/backend/monomorph.html).
2. Dataflow programming model, capturing the streaming message/data-driven nature of nodes within a system.
3. Reactive programming model with cumulative state, capturing the nature of stateful services (also applicable to front-end frameworks).
4. `#[no_std]` no-alloc support, using bounded static memory.
5. Easy-to-read surface syntax, embeddable in Rust.

The most recent version of the [DFIR docs are online](https://hydro.run/docs/dfir/#this-documentation), providing documentation and examples.

You can also check out the [DFIR Playground](https://hydro.run/docs/dfir/playground) to try out DFIR's surface syntax.
