# jxl-oxide
[![crates.io](https://img.shields.io/crates/v/jxl-oxide.svg)](https://crates.io/crates/jxl-oxide)
[![docs.rs](https://docs.rs/jxl-oxide/badge.svg)](https://docs.rs/crate/jxl-oxide/)
[![Build Status](https://img.shields.io/github/actions/workflow/status/tirr-c/jxl-oxide/build.yml?branch=main)](https://github.com/tirr-c/jxl-oxide/actions/workflows/build.yml?query=branch%3Amain)

A spec-conforming[^1] JPEG XL decoder written in pure Rust[^2].

jxl-oxide consists of small library crates (`jxl-bitstream`, `jxl-coding`, ...), a blanket library
crate `jxl-oxide`, and a binary crate `jxl-oxide-cli`. If you want to use jxl-oxide in a terminal,
install it using `cargo install`. Cargo will install two binaries, `jxl-dec` and `jxl-info`.

Current version of `jxl-oxide-cli` is 0.3.1.

```
cargo install jxl-oxide-cli
```

If you want to use it as a library, specify it in `Cargo.toml`:

```toml
[dependencies]
jxl-oxide = "0.5.2"
```

---

Dual-licensed under MIT and Apache 2.0.

[^1]: jxl-oxide passes all Level 5 tests and *almost* all Level 10 tests
[defined in the spec][conformance].
[^2]: Integration with Little CMS 2, which is written in C, can be enabled with `lcms2` feature.

[conformance]: https://github.com/libjxl/conformance
