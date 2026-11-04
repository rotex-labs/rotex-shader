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

## Part of Rotex

Rotex is a graphics HAL workspace. See [rotex-dev](https://github.com/rotex-labs/rotex-dev) for the architecture and the other crates.

## License

Licensed under [MIT](LICENSE).
