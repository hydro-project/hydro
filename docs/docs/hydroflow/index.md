---
sidebar_position: 1
---

# Introduction

[Hydroflow](https://github.com/hydro-project/hydroflow) is a compiler for low-latency 
dataflow programs, written in Rust. Hydroflow is the runtime library for the 
[Hydro language stack](./ecosystem.md), which is under development
as a complete compiler stack for distributed programming languages. 

Hydroflow is designed with two goals in mind:
- Expert developers can program Hydroflow directly to build components in a distributed system.
- Higher levels of the Hydro stack will offer friendlier languages with more abstractions, and treat Hydroflow as a compiler target.


Hydroflow provides a DSL—a *surface syntax*—embedded in Rust, which compiles to high-efficiency machine code. 
As the lowest level of the Hydro stack, Hydroflow
requires some knowledge of Rust to use. 
## This Book
This book will teach you how to set up your environment to get started with Hydroflow, and how to program in the Hydroflow surface syntax.

Keep in mind that Hydroflow is under active development and is constantly
changing. However the code in this book is tested with the Hydroflow library so should always be up-to-date.

If you have any questions, feel free to [create an issue on Github](https://github.com/hydro-project/hydroflow/issues/new).
