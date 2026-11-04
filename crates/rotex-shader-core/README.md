<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/rotex-labs/rotex-assets/55fa2474cb788a445c33b4ccf615eb8bbb9dbe64/brand/github/rotex-readme-banner-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/rotex-labs/rotex-assets/55fa2474cb788a445c33b4ccf615eb8bbb9dbe64/brand/github/rotex-readme-banner-light.svg">
    <img alt="Rotex: One API. Every GPU. Zero cost." src="https://raw.githubusercontent.com/rotex-labs/rotex-assets/55fa2474cb788a445c33b4ccf615eb8bbb9dbe64/brand/github/rotex-readme-banner-light.svg" width="1200">
  </picture>
</p>

# rotex-shader-core

The shader compilation contract.

> [!WARNING]
> **Beta, pre-1.0.** Rotex is under active development. APIs, behavior, and crate
> boundaries can change in any 0.x release. Do not depend on it for production work yet.

## Overview

`rotex-shader-core` defines how a compiler turns source into a shader package. It does not invoke a compiler backend. `rotex-shader-cooker` implements the trait.

## Responsibilities

- `ShaderCompiler`, `ShaderSource`, and `CompileOptions`
- `CompilerError`
- `merge_graphics_layout` for a graphics shader package

## Dependencies

`rotex-types`.

## Status

Version 0.2.0. No GPU API imports.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
