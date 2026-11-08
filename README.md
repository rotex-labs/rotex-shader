<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/rotex-labs/rotex-assets/55fa2474cb788a445c33b4ccf615eb8bbb9dbe64/brand/github/rotex-readme-banner-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/rotex-labs/rotex-assets/55fa2474cb788a445c33b4ccf615eb8bbb9dbe64/brand/github/rotex-readme-banner-light.svg">
    <img alt="Rotex: One API. Every GPU. Zero cost." src="https://raw.githubusercontent.com/rotex-labs/rotex-assets/55fa2474cb788a445c33b4ccf615eb8bbb9dbe64/brand/github/rotex-readme-banner-light.svg" width="1200">
  </picture>
</p>

# rotex-shader

The shader compilation contract and the text-shader cooker.

> [!WARNING]
> **Beta, pre-1.0.** Rotex is under active development. APIs, behavior, and crate
> boundaries can change in any 0.x release. Do not depend on it for production work yet.

## Overview

This repository defines how a compiler turns source into a shader package, then implements that contract for text shaders. `rotex-shader-core` does not invoke a compiler. `rotex-shader-cooker` reflects one naga module into WGSL, MSL, GLSL, and HLSL. Backends load those payloads. They do not compile source. This repository is separate so shaderc and naga are not on the HAL build.

## Crates

| Package | Description | Version |
|---------|-------------|---------|
| [`rotex-shader-core`](crates/rotex-shader-core/README.md) | The shader compilation contract | 0.3.0 |
| [`rotex-shader-cooker`](crates/rotex-shader-cooker/README.md) | Reflects shaders into the payloads backends load | 0.3.0 |

## Dependencies

`rotex-shader-cooker` depends on `rotex-shader-core` by path plus version inside this repository. Both depend on published `rotex-types`. A requirement that leaves the slice stays a crates.io version. Each package keeps its own version and its own release tag (`rotex-shader-core-v0.3.0`, `rotex-shader-cooker-v0.3.0`).

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](crates/rotex-shader-core/LICENSE-APACHE))
* MIT license ([LICENSE-MIT](crates/rotex-shader-core/LICENSE-MIT))

at your option. Each crate directory carries the same two files.
