# PNA Rust — Building Blocks 1
Read all the readings and perform all the exercises.

- **[Exercise: Write a Good CLI Program]**. Writing a CLI program in Rust. This 
  is a good warmup to the CLI program you'll be writing in this course, and the
  techniques used by this author may provide an interesting contrast to those we
  suggest. Follow along and write the same code. Can you reproduce their
  results?

- **[Reading: The Cargo manifest format]**. A single page in [The Cargo Book],
  this will give you an idea of how your project can be customized a bit if you
  so choose. This is a page you will come back to repeatedly as a Rust
  programmer.

- **[Reading: Cargo environment variables]**. Also from The Cargo Book, and also
  a page that you will see many times in the future. Environment variables are
  one way that it communicates with rustc, allowing it to set the various
  [`env!`] macros at build time, in both your program source code and build
  scripts. It is also a way for scripts and other systems to communicate to
  Cargo.

- **[Reading: Rust API Guidelines: Documentation]**. The Rust project is
  opinionated about how Rust source is written. This page is on how to document
  Rust projects, but the whole book is worth reading. These are written by
  experienced Rust developers, but are in an incomplete state. Note the GitHub
  organization it belongs to &mdash; [`rust-lang-nursery`]. It contains many
  interesting projects.


[Reading: Rust API Guidelines: Documentation]: https://rust-lang-nursery.github.io/api-guidelines/documentation.html
[Reading: The Cargo manifest format]: https://doc.rust-lang.org/cargo/reference/manifest.html
[Reading: Cargo environment variables]: https://doc.rust-lang.org/cargo/reference/environment-variables.html
[The Cargo Book]: https://doc.rust-lang.org/cargo/reference/manifest.html
[`env!`]: https://doc.rust-lang.org/std/macro.env.html
[`rust-lang-nursery`]: https://github.com/rust-lang-nursery
[Reading: The rustup documentation]: https://github.com/rust-lang/rustup.rs/blob/master/README.md
[Exercise: Write a Good CLI Program]: https://qiita.com/tigercosmos/items/678f39b1209e60843cc3

## The Cargo manifest format
The Cargo.toml file for each package is called its manifest. It is written in the TOML format. Every manifest file consists of the following sections:

* cargo-features — Unstable, nightly-only features.
* [package] — Defines a package.
  * name — The name of the package.
  * version — The version of the package.
  * authors — The authors of the package.
  * edition — The Rust edition.
  * rust-version — The minimal supported Rust version.
  * description — A description of the package.
  * documentation — URL of the package documentation.
  * readme — Path to the package's README file.*
  * homepage — URL of the package homepage.
  * repository — URL of the package source repository.
  * license — The package license.
  * license-file — Path to the text of the license.
  * keywords — Keywords for the package.
  * categories — Categories of the package.
  * workspace — Path to the workspace for the package.
  * build — Path to the package build script.
  * links — Name of the native library the package links with.
  * exclude — Files to exclude when publishing.
  * include — Files to include when publishing.
  * publish — Can be used to prevent publishing the package.
  * metadata — Extra settings for external tools.
  * default-run — The default binary to run by cargo run.
  * autobins — Disables binary auto discovery.
  * autoexamples — Disables example auto discovery.
  * autotests — Disables test auto discovery.
  * autobenches — Disables bench auto discovery.
  * resolver — Sets the dependency resolver to use.
* Target tables: (see configuration for settings)
  * `[lib]` — Library target settings.
  * `[[bin]]` — Binary target settings.
  * `[[example]]` — Example target settings.
  * `[[test]]` — Test target settings.
  * `[[bench]]` — Benchmark target settings.
* Dependency tables:
  * [dependencies] — Package library dependencies.
  * [dev-dependencies] — Dependencies for examples, tests, and benchmarks.
  * [build-dependencies] — Dependencies for build scripts.
  * [target] — Platform-specific dependencies.
* [badges] — Badges to display on a registry.
* [features] — Conditional compilation features.
* [patch] — Override dependencies.
* [replace] — Override dependencies (deprecated).
* [profile] — Compiler settings and optimizations.

## Rust API Guidelines: Documentation
* All items have a rustdoc example： an example is often intended to show why someone would want to use the item.
* Examples use ?, not try!, not unwrap: The lines beginning with # are compiled by cargo test when building the example but will not appear in user-visible rustdoc.
* Function docs include error, panic, and safety considerations.
* Prose contains hyperlinks to relevant things.
* Cargo.toml includes all common metadata.

The [package] section of Cargo.toml should include the following values:
* authors
* description
* license
* repository
* keywords
* categories

In addition, there are two optional metadata fields:
* documentation
* homepage
  
By default, crates.io links to documentation for the crate on docs.rs. **The documentation metadata only needs to be set if the documentation is hosted somewhere other than docs.rs**, for example because the crate links against a shared library that is not available in the build environment of docs.rs.

**The homepage metadata should only be set if there is a unique website for the crate other than the source repository or API documentation.** Do not make homepage redundant with either the documentation or repository values. For example, serde sets homepage to https://serde.rs, a dedicated website.

* Realse notes document all significant changes.
* Rustdoc does not show unhelpful implementation details. You can use `#[doc(hidden)]`.