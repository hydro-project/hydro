# DFIR

DFIR (Dataflow Intermediate Representation) is a small language and compiler for low-latency
dataflow programs, written in Rust. DFIR serves as dataflow representation and runtime execution library for the
[Hydro framework](https://hydro.run/docs/hydro/), which is under development
as a complete framework for distributed programming.

DFIR is designed with two use cases in mind:
  - Expert developers can program directly in the DFIR language to build individual components that can interoperate in a distributed program or service.
  - Higher levels of the Hydro stack will offer higher-level abstractions and DSLs, and treat DFIR as a compiler target.

DFIR is targeted at supporting the following unique features:
  1. An easy-to-read flow syntax, embeddedable in Rust.
  2. Dataflow programming model, capturing the streaming message/data-driven nature of distributed computational services.
  3. Reactive programming model with cumulative state, capturing the nature of both stateful distributed services and front-end frameworks.
  4. Extremely low-latency execution via [Rust monomorphization](https://rustc-dev-guide.rust-lang.org/backend/monomorph.html), while maintaining high throughput.

DFIR's language—the DFIR *flow syntax*—is embeddedable in Rust, which compiles DFIR code to high-efficiency machine code.
As the lowest level of the Hydro stack, DFIR requires some knowledge of Rust to use.

The most recent release of the [DFIR Book docs](https://hydro.run/docs/dfir/#this-book) are online, providing documentation and numerous annotated examples.

You can also check out the [DFIR Playground](https://hydro.run/docs/dfir/playground) to see DFIR's flow syntax in action!
