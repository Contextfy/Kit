# Project Context

## Purpose

Contextfy/Kit is a high-performance AI Context Orchestration Engine designed to solve the "knowledge gap" and "black box retrieval" problems faced by AI Agents in vertical domain development. We transform unstructured technical documentation (Markdown, API Docs) into standardized, distributable, AI-native Context Packs and provide a high-performance runtime environment for upper-layer applications (CLI, MCP Server) to call.

**Vision:** "Context as Code."

## Tech Stack

### Core (Rust)
- **Parsing**: `pulldown-cmark` - Markdown AST parsing
- **Storage**: `LanceDB` - Vector database with Arrow format for zero-copy reads
- **Full-Text Search**: `Tantivy` - BM25 keyword search
- **Embeddings**: `FastEmbed` - Local ONNX models (offline-first, no API costs)
- **Async Runtime**: `Tokio` - Async/await concurrency model

### Bridge Layer (FFI)
- **Node.js**: `napi-rs` - High-performance bindings for JavaScript/TypeScript
- **Python**: `pyo3` - Native Python extensions

### Web UI
- **Frontend**: Next.js + TypeScript
- **Backend**: Axum + Tokio (Rust)
- **Visualization**: D3.js / Cytoscape.js for knowledge graphs

### System & Infrastructure
- **CLI**: Built with Rust and published via cargo
- **Packaging**: Monorepo structure with workspace management
- **Build System**: Cargo workspaces

## Project Conventions

### Code Style

#### Rust
- Follow Rust standard formatting: `cargo fmt`
- Use `cargo clippy` for linting
- Error handling: Use `Result<T, E>` throughout, avoid panics in core logic
- Naming: `snake_case` for variables/functions, `PascalCase` for types/structs
- Async: Prefer `async fn` over blocking I/O operations
- Documentation: Use `///` for public APIs with examples

#### TypeScript/JavaScript
- TypeScript strict mode enabled
- Functional style where appropriate
- Clear type definitions for all public APIs

### Architecture Patterns

1. **Headless First**: Core logic (`packages/core`) must be completely decoupled from presentation layer (CLI/Web/MCP). Core crate should not contain any UI code.

2. **Zero-Copy Read**: Utilize LanceDB and Arrow format characteristics to minimize memory copies during retrieval, ensuring high performance on low-end devices.

3. **Fault Isolation**: A Context Pack corruption or parsing failure should not crash the entire engine (Panic Safe design).

4. **Observability Built-in**: Retrieval chains must expose Trace IDs and detailed scoring logs, not just return results.

5. **Two-Stage Retrieval**:
   - **Scout (侦察)**: Returns only summary and scores, latency < 20ms
   - **Inspect (检视)**: Loads full content on-demand, avoiding Token waste
   - Hybrid retrieval: Vector Search + BM25

6. **Monorepo Structure**:
   ```
   Contextfy/Kit
   ├── packages/core/          # Core engine (Rust)
   ├── packages/bridge/        # FFI glue layer
   ├── packages/web/           # Visualization dashboard
   └── docs/                  # Project documentation
   ```

### Testing Strategy

- **Performance Benchmarks**: Scout latency < 20ms for 100MB text knowledge base
- **Accuracy Metrics**: Top-3 recall rate > 90% on test sets
- **Cold Start**: Build time < 5 minutes from init to AI-callable
- Tests should cover: parsing correctness, retrieval accuracy, incremental builds

### Git Workflow

- **Conventional Commits** (implied by professional standards)
- Branching: `main` for stable, feature branches for development
- Use pull requests for all changes
- Before merging: validation, build, and tests must pass

## Domain Context

### Core Concepts

1. **Context Pack**: Similar to Docker images, versioned knowledge packages containing:
   - Compiled documentation (Markdown → LanceDB)
   - Manifest metadata (version, sources, indexing config)
   - Physical isolation via namespace

2. **Two-Stage Retrieval Primitives**:
   - `scout(query, limit) -> Vec<Brief>`: Fast reconnaissance on summaries
   - `inspect(ids) -> Vec<Details>`: Load full content on-demand

3. **Namespace Isolation**: Multiple packs can be loaded simultaneously (e.g., `fabric-1.21` and `java-std-lib`) without interference.

4. **Incremental Compilation**: Build system skips unchanged chapters based on file hash.

### AI Agent Integration

The engine is designed to be called by AI Agents (Claude Code, OpenCode) via:
- **CLI**: `contextfy scout`, `contextfy inspect` commands
- **FFI**: Node.js and Python bindings for direct library usage
- **Skills**: Prompt-level integration to guide AI workflow (e.g., "Always call `contextfy scout` before writing code")

### MVP Use Case: Minecraft Bedrock Addon Development

The first validation scenario is helping AI build high-quality Minecraft Bedrock Addons:
- User asks: "Create a red 'Healing Stone' block that restores 2 HP per second when players stand on it."
- System automatically:
  1. Creates compliant BP/RP directory structure
  2. Registers textures and block definitions in RP
  3. Retrieves accurate `@minecraft/server` API documentation
  4. Writes TypeScript script logic
  5. No manual intervention needed - generated code works in-game immediately

## Important Constraints

### Technical Constraints
- **Local-First**: No data uploaded to cloud, all processing happens locally
- **Offline Capable**: Must work without internet (local ONNX models for embeddings)
- **Performance**: Scout latency must be < 20ms even for large knowledge bases
- **Memory Efficiency**: Must run on low-end devices; minimize memory copies
- **Panic Safe**: Core engine must not panic on malformed input (DoS protection)

### Design Constraints
- **Simplicity First**: Default to <100 lines of new code, single-file implementations
- **Proven Patterns**: Choose boring, proven patterns over clever abstractions
- **Complexity Triggers**: Only add complexity with performance data, concrete scale requirements, or multiple proven use cases

### Security Constraints
- **Path Traversal**: Compiler must strictly limit reads to `manifest.json` defined root directory
- **Resource Limits**: Limit LanceDB memory mapping to prevent OOM
- **Input Sanitization**: Prevent maliciously crafted Markdown from causing parser crashes

## External Dependencies

### Core Runtime
- **LanceDB**: Vector database for embeddings storage
- **Tantivy**: Full-text search engine (BM25)
- **FastEmbed**: ONNX runtime for local embeddings (BGE-small-en model)

### Build & Tooling
- **Rust**: `cargo`, `rustfmt`, `clippy`
- **napi-rs**: Node.js bindings generator
- **PyO3**: Python bindings generator

### Web Stack
- **Next.js**: Frontend framework
- **Axum**: Rust web framework
- **Tokio**: Async runtime

### Data Sources (for MVP)
- **MicrosoftDocs/minecraft-creator**: Official Minecraft Bedrock documentation (Markdown source)
- **TypeScript definition files (.d.ts)**: Indexed as plain text code blocks for accurate type retrieval
