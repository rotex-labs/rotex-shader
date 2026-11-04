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

Version 0.2.0. `SlangCompiler` is public and returns unsupported. This crate does not claim Slang support.

## Part of Rotex

Rotex is a graphics HAL workspace. See [rotex-dev](https://github.com/rotex-labs/rotex-dev) for the architecture and the other crates.

## License

Licensed under [MIT](LICENSE).
