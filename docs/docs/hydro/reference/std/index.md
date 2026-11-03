# Standard Library

[`hydro_std`](rust:hydro_std) is a crate of reusable building blocks for Hydro programs. Two of those building blocks are documented here:

- [Quorums](./quorum.md) count `Ok` and `Err` replies per key until enough successes have arrived.
- [Request and Response](./request-response.md) keeps metadata from request time until the matching response arrives.

Membership changes are tracked by the `hydro_std::membership` module and are not covered here.
