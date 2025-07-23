# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

This is a Rust workspace project using Cargo. Common commands:

- `cargo build` - Build all workspace members
- `cargo test` - Run tests for all workspace members  
- `cargo test -p zenith-task` - Run tests for specific package
- `cargo run --bin zenith-sandbox` - Run the sandbox application
- `cargo run --example model` - Run the model example from zenith-sandbox
- `cargo run --example test` - Run the task system example from zenith-task
- `cargo check` - Fast compile check without building binaries

## Project Architecture

Zenith is a modular Vulkan-based rendering engine built as a Rust workspace with clear separation of concerns:

### Core Modules

- **zenith**: Main engine crate containing the application framework (`app.rs`, `engine.rs`, `main_loop.rs`)
- **zenith-core**: Foundation utilities including asset loading (GLTF), collections, and logging
- **zenith-render**: Low-level rendering abstractions (device management, pipeline cache, shader compilation)
- **zenith-renderer**: High-level rendering components (mesh renderers, triangle renderer)
- **zenith-rendergraph**: Render graph system for organizing rendering passes (builder pattern, resource management)
- **zenith-task**: High-performance task graph system with thread pool scheduling and async task handles
- **zenith-sandbox**: Testing and example application
- **zenith-build**: Build utilities

### Key Technologies

- **Graphics**: wgpu for Vulkan/Metal/DX12 abstraction, naga_oil for shader composition
- **Math**: glam for 3D math operations
- **Asset Loading**: gltf crate for 3D model loading
- **Concurrency**: Custom task system in zenith-task with crossbeam-queue and threadpool
- **Window Management**: winit for cross-platform windowing

### Architecture Patterns

The project follows a modular design where each crate has specific responsibilities. The render graph system uses a builder pattern for constructing rendering pipelines. The task system implements a work-stealing thread pool with async handles for task results.

Asset loading is handled through zenith-core with support for GLTF models in the `content/mesh/` directory.

## Testing

- Run all tests: `cargo test`
- Test specific package: `cargo test -p [package-name]`
- The zenith-task crate has comprehensive test coverage including concurrent execution tests

## Examples

- Model rendering: `cargo run --example model` (from zenith-sandbox)
- Task system: `cargo run --example test` (from zenith-task)