# 🧭 RasgaPDF Documentation Architecture (Diátaxis Framework)

RasgaPDF's documentation is organized according to the **[Diátaxis documentation framework](https://diataxis.fr/)**, a systematic approach to technical documentation that solves four distinct user needs across two axes:

```
                      PRACTICAL / ACTION-ORIENTED
                                   ▲
                                   │
              TUTORIALS            │         HOW-TO GUIDES
         (Learning-oriented)       │       (Problem-oriented)
                                   │
  Learning by doing a guided path  │  Solving specific real-world tasks
  First run & hands-on success     │  Practical recipes & workflows
                                   │
◄──────────────────────────────────┼──────────────────────────────────►
  THEORY / COGNITION               │                   WORK / TASK
                                   │
             EXPLANATION           │           REFERENCE
       (Understanding-oriented)    │     (Information-oriented)
                                   │
  Architecture, design decisions   │  Exact specifications & schemas
  Trade-offs, history & rationale  │  APIs, FFI contracts & flags
                                   │
                                   ▼
                     THEORETICAL / KNOWLEDGE-ORIENTED
```

---

## The Four Documentation Quadrants

### 1. 🎓 Tutorials (Learning-Oriented)
Designed for newcomers who want a guided, hands-on path to their first success with RasgaPDF.

| Document | Description | Target Runtime |
| :--- | :--- | :--- |
| **[Quickstart Tutorial](tutorials/quickstart.md)** | 5-minute guided walkthrough from installation to first spatial parse and table extraction. | Python, Node.js, Rust, CLI |

---

### 2. 🛠️ How-To Guides (Problem-Oriented)
Practical, step-by-step recipes addressing specific real-world tasks and use cases.

| Guide | Problem Solved | Key Tech |
| :--- | :--- | :--- |
| **[Extract Tables to CSV & DataFrames](how-to/extract-tables-to-csv.md)** | Detect PDF matrix tables and export them directly to `.csv` files or pandas DataFrames without GPU overhead. | Python (pandas), Node.js, Rust |
| **[Spatial RAG & Bounding Box Citations](how-to/spatial-rag-coordinates.md)** | Build ground-truth visual citations and bounding-box highlight overlays for Retrieval-Augmented Generation. | Python, LangChain, PDF.js |
| **[Pure Rust WebAssembly in Browser](how-to/pure-rust-wasm.md)** | Run zero-server, 100% private PDF parsing directly inside the web browser via `rasgapdf.wasm`. | WebAssembly, JavaScript |

---

### 3. 📖 Reference (Information-Oriented)
Exhaustive, structured technical descriptions, field schemas, FFI symbols, and command-line flags.

| Reference | Scope & Purpose |
| :--- | :--- |
| **[Reference Index](reference/README.md)** | Overview of all technical specifications and supported ISO 32000-1 features. |
| **[Spatial JSON Schema Specification](SCHEMA.md)** | Detailed field specifications for Mode 1 (`RawSpatialDocument`) and Mode 2 (`SpatialDocument`). |
| **[C-ABI Header (`include/rasgapdf.h`)](../include/rasgapdf.h)** | C99 FFI declarations, parameter types, heap allocation, and memory ownership contracts. |
| **[CLI Options Reference](reference/README.md#3-cli-command-line-reference)** | Complete command-line arguments, format options, and exit codes. |

---

### 4. 🏛️ Explanation (Understanding-Oriented)
In-depth discussions on system design, technical background, design trade-offs, and architecture.

| Document | Key Concepts Explained |
| :--- | :--- |
| **[Explanation Index](explanation/README.md)** | Rationale for pure Rust ISO 32000-1 engine, concurrency models, and table geometry vs ML models. |
| **[Architecture & Technical Design](ARCHITECTURE.md)** | End-to-end system flowchart, Rayon thread-pool parallelization, polyglot C-ABI architecture, and performance benchmarks. |

---

## Machine-Readable Context for AI Agents & LLMs

For autonomous coding agents, LLMs, and automated toolchains:
- **`llms.txt`**: Concise project summary, performance metrics, and primary API quickstarts.
- **`llms-full.txt`**: Complete technical context, API definitions across all 5 languages, and full JSON schemas.
- **`AGENTS.md`**: Repository layout, development guidelines, and agent instructions.
