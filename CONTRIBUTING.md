# Contributing to RasgaPDF

Thank you for your interest in contributing to **RasgaPDF**! We welcome contributions from developers across all ecosystems (Rust, Python, Node.js, Go, C#).

---

## 1. Code of Conduct

This project and everyone participating in it is governed by the [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

---

## 2. Architecture Overview

- **Core Engine:** Written in pure, high-performance Rust (`src/`). It handles geometry extraction, parallel page processing via `rayon`, and spatial table detection.
- **FFI Layer:** Exposes a clean, cross-platform C-ABI in `src/ffi.rs` and `include/rasgapdf.h`.
- **Language Bindings:**
  - Python: `bindings/python/`
  - Node.js: `bindings/nodejs/`
  - Go: `bindings/go/`
  - C# (.NET 8): `bindings/csharp/`
- **Interactive Testing:** Visual Web App in `space_hf/` and `server_local.py`.

---

## 3. Development Workflow

### Prerequisites
- [Rust](https://www.rust-lang.org/) (1.75+ or 2021 edition)
- `cargo`
- Python 3.8+ (for Python binding & Gradio app)
- Node.js 18+ (for Node.js binding)
- Go 1.20+ (for Go binding)
- .NET 8 SDK (for C# binding)

### Building the Project
```bash
# Clone the repository
git clone https://huggingface.co/martinsmessias/rasgapdf
cd rasgapdf

# Build Rust library and CLI in release mode
cargo build --release

# Run CLI tests
./target/release/rasgapdf --help
```

### Running Tests
```bash
# Rust core tests
cargo test

# Python binding test
python3 bindings/python/test_python.py

# Node.js binding test
node bindings/nodejs/index.js

# Go binding test
cd bindings/go && go run cmd/main.go
```

---

## 4. Submitting a Pull Request

1. Fork the repo and create your branch from `main`:
   ```bash
   git checkout -b feature/my-amazing-feature
   ```
2. Commit your changes with clear, descriptive commit messages:
   ```bash
   git commit -m "feat(core): add font family extraction to spatial spans"
   ```
3. Push to your fork:
   ```bash
   git push origin feature/my-amazing-feature
   ```
4. Open a Pull Request on GitHub with a description of your changes and benchmark impact.

Thank you for helping make PDF parsing fast, accurate, and truly spatial!
