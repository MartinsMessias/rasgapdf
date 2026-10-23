---
title: RasgaPDF 2D Spatial PDF Parser
emoji: ⚡
colorFrom: blue
colorTo: indigo
sdk: docker
app_port: 7860
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
- **⚡ Native Rust Engine (`librasgapdf.so`):** Powered by the official Rust spatial PDF parser backend running in a dedicated Docker container. Provides microsecond-level extraction of text spans, tables, bounding boxes, drawings, and images.
- **🛡️ Client-Side Fallback:** If the backend is unreachable or under heavy load, the visualizer automatically falls back to client-side compute directly inside your browser via PDF.js.
- **Want to run on your own Hugging Face hardware quota?** Click the **[Duplicate this Space](https://huggingface.co/spaces/martinsmessias/rasgapdf?duplicate=true)** button to clone this Space to your personal Hugging Face account and allocate your own CPU/GPU hardware quota.

### ⚠️ Known limitation: protected PDFs
- **DRM (Adobe ADEPT / `EBX_HANDLER`)**: typical of e-books. The content is locked by the store's key, so no PDF library can read it without removing the DRM. The visualizer reports this explicitly instead of failing silently.
- **Password-protected**: detected and reported; remove the password (export an unprotected copy) and try again.
