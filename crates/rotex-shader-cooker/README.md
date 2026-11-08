<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/rotex-labs/rotex-assets/55fa2474cb788a445c33b4ccf615eb8bbb9dbe64/brand/docs/crate-headers/dark/rotex-shader-cooker-header.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/rotex-labs/rotex-assets/55fa2474cb788a445c33b4ccf615eb8bbb9dbe64/brand/docs/crate-headers/light/rotex-shader-cooker-header.svg">
    <img alt="rotex-shader-cooker" src="https://raw.githubusercontent.com/rotex-labs/rotex-assets/55fa2474cb788a445c33b4ccf615eb8bbb9dbe64/brand/docs/crate-headers/light/rotex-shader-cooker-header.svg" width="1200">
  </picture>
</p>

# rotex-shader-cooker

Reflects shaders into the payloads backends load.

> [!WARNING]
> **Beta, pre-1.0.** Rotex is under active development. APIs, behavior, and crate
> boundaries can change in any 0.x release. Do not depend on it for production work yet.

## Overview

`rotex-shader-cooker` compiles text shaders and writes serialized shader packages. One naga module is reflected into WGSL, MSL, GLSL, and HLSL. Backends load those payloads. They do not compile source.

## Responsibilities

- `GlslCompiler` and `WgslCompiler`
- SPIR-V and WGSL reflection into a shared layout
- Read and write of serialized shader packages

## Dependencies

`rotex-shader-core` and `rotex-types`.

## Status

Version 0.3.0. `SlangCompiler` is public and returns unsupported. This crate does not claim Slang support.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
