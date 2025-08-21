<a href="https://hydro.run"><h1 align="center">
    <img src="https://raw.githubusercontent.com/hydro-project/hydro/main/docs/static/img/hydro-logo.svg" width="400" alt='"hf"'>
</h1></a>
<p align="center">
    <a href="https://crates.io/crates/hydro_lang"><img src="https://img.shields.io/crates/v/hydro_lang?style=flat-square&logo=rust" alt="Crates.io"></a>
    <a href="https://docs.rs/hydro_lang/"><img src="https://img.shields.io/badge/docs.rs-Hydro-blue?style=flat-square&logo=read-the-docs&logoColor=white" alt="Docs.rs"></a>
</p>

Hydro is a high-level distributed programming framework for Rust. Hydro can help you quickly write scalable distributed services that are correct by construction. Much like Rust helps with memory safety, Hydro helps with [**distributed safety**](https://hydro.run/docs/hydro/correctness).

Hydro integrates naturally with standard Rust constructs and IDEs, providing types and programming constructs for ensuring distributed safety. Under the covers, Hydro is powered by the Dataflow Intermediate Representation (DFIR), a compiler and low-level runtime for stream processing. DFIR enables automatic vectorization and efficient scheduling without restricting your application logic.

<b>Get started today at <a href="https://hydro.run">hydro.run</a>!</b>

# Learn More
- **Docs**: There are docs for the [high-level Hydro framework](https://hydro.run/docs/hydro/) and the low-level dataflow IR, [DFIR](https://hydro.run/docs/dfir), as well as the [Hydro Deploy](https://hydro.run/docs/deploy) framework for launching Hydro programs.

- **Research Papers**: Our [research publications](https://hydro.run/research) are available on the project website. Some notable selections:
    - The original Hydro vision paper from CIDR 2021: [New Directions in Cloud Programming](https://hydro.run/papers/new-directions.pdf)
    - The first paper on optimizations from SIGMOD 2024: [Optimizing Distributed Protocols with Query Rewrites](https://hydro.run/papers/david-sigmod-2024.pdf)
    - The first paper on Hydro's formal semantics to appear in POPL 2025: [Flo: a Semantic Foundation for Progressive Stream Processing](https://arxiv.org/abs/2411.08274)

# Contributing

For Hydro development setup and contribution info, see [CONTRIBUTING.md](CONTRIBUTING.md).
