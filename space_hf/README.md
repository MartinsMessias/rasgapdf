---
title: RasgaPDF 2D Spatial PDF Parser
emoji: ⚡
colorFrom: blue
colorTo: indigo
sdk: static
pinned: false
license: agpl-3.0
tags:
- pdf
- spatial
- rust
- layout-analysis
- document-parsing
- bounding-box
---

# ⚡ RasgaPDF // 2D Spatial PDF Parser (Web Visualizer)

<div align="center">

[![Duplicate this Space](https://huggingface.co/datasets/huggingface/badges/resolve/main/duplicate-this-space-md.svg)](https://huggingface.co/spaces/martinsmessias/rasgapdf?duplicate=true)
[![PyPI version](https://img.shields.io/pypi/v/rasgapdf.svg?color=blue&logo=pypi&logoColor=white)](https://pypi.org/project/rasgapdf/)
[![Crates.io](https://img.shields.io/crates/v/rasgapdf.svg?color=orange&logo=rust)](https://crates.io/crates/rasgapdf)
[![npm version](https://img.shields.io/npm/v/rasgapdf.svg?color=red&logo=npm)](https://www.npmjs.com/package/rasgapdf)
[![Model Repo](https://img.shields.io/badge/%F0%9F%A4%97%20Hub-Model%20Repository-blue)](https://huggingface.co/martinsmessias/rasgapdf)

</div>

### 🚀 Dual-Engine Architecture: Native Rust + Client-Side Fallback
- **⚡ Native Rust Engine (`librasgapdf.so`):** Powered by the official Rust spatial PDF parser backend. Provides microsecond-level extraction of text spans, tables, bounding boxes, drawings, and images. It needs a Python server, so it only runs when this Space is deployed with the Docker SDK (Hugging Face requires a paid plan for Docker Spaces).
- **🛡️ Client-Side Fallback:** When that backend is unreachable — as on this free static deploy — the visualizer extracts everything directly inside your browser via PDF.js, with the same overlays and JSON output.
- **Want to run the native backend on your own Hugging Face hardware quota?** Click the **[Duplicate this Space](https://huggingface.co/spaces/martinsmessias/rasgapdf?duplicate=true)** button, switch the SDK to Docker and allocate your own hardware.

### ⚠️ Known limitation: protected PDFs
- **DRM (Adobe ADEPT / `EBX_HANDLER`)**: typical of e-books. The content is locked by the store's key, so no PDF library can read it without removing the DRM. The visualizer reports this explicitly instead of failing silently.
- **Password-protected**: detected and reported; remove the password (export an unprotected copy) and try again.
