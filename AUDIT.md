# RasgaPDF — Repository Audit Report

**Date:** 2026-10-30
**Repository:** `MartinsMessias/rasgapdf` (branch `main`, HEAD `a5fa6a8`)
**Scope:** every surface of the repository — Rust core engine, FFI/bindings (Python, Node.js, Go, C#), documentation & web visualizer, repository hygiene, CI/CD, tests.
**Method:** static code review of all tracked source, cross-checking docs against code, dependency/config inspection, and four parallel audit passes (core engine, bindings/FFI, docs/web, hygiene/security).

This report lists **confirmed** issues with file:line references. Items marked `(unverified)` could not be live-tested in this environment.

---

## 0. Executive summary (top priorities)

1. **XRef revision overwrite** (`src/engine/xref.rs:195,267,277`) — incremental updates resolve to stale objects. Fix with `entry().or_insert()`.
2. **Unbounded recursion** on crafted PDFs (`src/engine/doc.rs:72-79,100-118`; `src/engine/xref.rs:123-135`) — stack overflow DoS.
3. **Decompression bomb** (`src/engine/flate.rs:42-48`) — `read_to_end` with no cap.
4. **Shift overflow** in XRef stream decoding (`src/engine/xref.rs:293-297`) — UB on crafted `/W`.
5. **CID width range expansion** (`src/engine/font.rs:40-43`) — 65k inserts + possible overflow.
6. **WASM allocator unsoundness** (`src/ffi.rs:266-279`) — capacity mismatch on free.
7. **Stale/misleading documentation** — MuPDF linkage claims after the pure-Rust migration; version `0.2.6` in LLM docs; AGENTS.md paths/scripts that do not exist.
8. **Repository bloat** — ~8 MB native binaries in Git history; 6 GB local `target/`; `.cargo/config.toml` with a machine-specific path.
9. **CI gaps** — Node test has no `npm install`; Go test swallows failures (`|| true`); CodeQL skips Rust/C#/Go; Dependabot omits Go/NuGet/Docker.
10. **Diátaxis coverage is thin** — only reference/explanation; no tutorials or how-to guides.

---

## 1. Diátaxis documentation coverage

Diátaxis organizes technical docs into four quadrants. Current state:

| Quadrant | Status | Evidence |
|---|---|---|
| **Tutorials** (learning-oriented) | Missing | No `docs/tutorials/`. README has quickstarts but no guided first-run walkthrough. |
| **How-to guides** (task-oriented) | Missing | No `docs/how-to/`. Issues #14, #15, #17, #18 explicitly request task recipes (RAG notebook, LlamaIndex citation, CLI Markdown output, benchmarks). |
| **Reference** (information-oriented) | Present | `docs/SCHEMA.md` (JSON schema), `docs/ARCHITECTURE.md` (engine internals), `include/rasgapdf.h`. |
| **Explanation** (understanding-oriented) | Present but stale | `docs/ARCHITECTURE.md` still references MuPDF DTOs after the pure-Rust migration. |

**Recommendation:** create `docs/tutorials/`, `docs/how-to/`, `docs/reference/`, `docs/explanation/` and add a Diátaxis landing page. Reconcile reference docs with the pure-Rust engine (see §4).

---

## 2. Core Rust engine

### 2.1 Correctness bugs

- **XRef entries overwrite newer revisions** — `src/engine/xref.rs:195,267,277` use `table.entries.insert(...)`. Since parsing walks from newest xref to oldest via `/Prev`, older revisions overwrite newer ones. Incrementally updated PDFs resolve to stale objects. Fix: `entry(...).or_insert(...)`.
- **Indirect `/Length` not resolved** — `src/engine/parser.rs:166-167` only handles `PdfObject::Integer`. Many PDFs use an indirect reference; fallback `endstream` substring search can truncate binary streams containing that marker. Fix: resolve `Length` through the XRef table before reading.
- **Generator mismatch ignored** — `src/engine/xref.rs:95-135` `get_object` does not compare `id.generation` against the stored entry's generation.
- **`XRefTable::entries` keyed only by object number** — `src/engine/xref.rs:13-14` `HashMap<u32, XRefEntry>` collapses generations. Key by `(u32, u16)` if generation-sensitive resolution is needed.
- **WebAssembly allocator unsound** — `src/ffi.rs:266-279` `wasm_alloc` uses `Vec::with_capacity(size)` (capacity may exceed `size`), then `wasm_free` reconstructs with `Vec::from_raw_parts(ptr, 0, size)`. Mismatched capacity → UB on drop; `size == 0` unhandled. Fix: exact allocation with stored size or document capacity contract.

### 2.2 Security / DoS

- **Unbounded recursion** (stack overflow) — `src/engine/doc.rs:72-79` (`resolve`), `src/engine/doc.rs:100-118` (`traverse_pages_node`), `src/engine/xref.rs:123-135` (`get_object` compressed branch). Cyclic object/page trees exhaust the stack. Fix: depth limit + visited set.
- **Decompression bomb** — `src/engine/flate.rs:42-48` `decompress_zlib` calls `read_to_end` with no output cap. Fix: bounded streaming read with configurable limit.
- **Shift overflow** — `src/engine/xref.rs:293-297` `read_int` shifts `u64` left by 8 per byte; if `/W` requests >8 bytes/field, shift ≥64 (UB). Fix: validate `w0/w1/w2 <= 8` or use `checked_shl`.
- **CID width range expansion** — `src/engine/font.rs:40-43` `add_cid_width_range` loops `start_cid..=end_cid` inserting each (65k entries for `0 65535 500`); `add_cid_width_entry` at `:34-37` can overflow on large arrays. Fix: store ranges in a separate map; checked arithmetic.
- **Unbounded graphics-state stack** — `src/engine/content.rs:160-161` pushes `GraphicsState` on every `q` with no cap. Fix: cap depth, error on overflow.
- **`ffi::create_error_json` panic on NUL** — `src/ffi.rs:144-148` `CString::new(s).unwrap()` panics if the message contains `\0`. Sanitize NUL bytes.
- **`rasgapdf_extract_text_bytes` swallows errors** — `src/ffi.rs:170-183` returns empty string on parse failure with no status. Return error JSON or out-parameter status.

### 2.3 Performance pitfalls

- `src/engine/xref.rs:202` `find_subsequence` is O(n·m) over the whole stream on every fallback (hot on malformed PDFs).
- `src/engine/font.rs:40-43` range expansion is both DoS and perf issue.
- `src/engine/doc.rs:72-79` re-parses uncompressed objects on every reference; `obj_cache` is only used for object streams. Caching all resolved objects reduces repeated parse cost.
- `src/engine/content.rs:104-118` appends every Form XObject in `/Resources` regardless of whether it is invoked by `Do`, duplicating content and inflating parse time.
- `rayon = "1.10"` (`Cargo.toml:14`) is declared but unused anywhere in `src/` (marketing claim "powered by Rayon" is inaccurate). Either implement parallel page processing or remove the dependency.

### 2.4 Unsafe / FFI boundary

All `unsafe` is confined to `src/ffi.rs` (expected C-ABI boundary). Issues:
- Exported functions dereference caller pointers but are **not marked `unsafe` and lack `# Safety` docs** (clippy `not_unsafe_ptr_derefs`). Document the non-null/NUL-terminated/valid-length contract.
- `rasgapdf_version` (`ffi.rs:130-133`) returns a pointer into a static string that must never be freed by callers — document this.
- `panic = "abort"` (`Cargo.toml:23`) means any panic in the FFI path aborts the host process; acceptable but should be documented.

### 2.5 Minor code-quality

- `src/engine/lexer.rs:130` `read_name` hex condition `pos + 2 < len` is correct but subtle; prefer `pos + 3 <= len`.
- `src/engine/font.rs:148-152` `cp1252_to_char` leaves `0x81/0x8D/0x8F/0x90/0x9D` as control characters (undefined in Win-1252); map to `U+FFFD` or skip.
- `src/engine/content.rs:257-263` and `:289-293` collapsible match/if-let (clippy).
- `src/engine/doc.rs:38-39` `get_elapsed_ms`/`now_instant` trivial wrappers (harmless).
- `src/engine/xref.rs:128` `obj_stream_cache.get(stream_num).unwrap()` — fragile; use `expect` with message.
- AGENTS.md references non-existent files (`src/graphics.rs`, `src/schema.rs`) and stale module names.

### 2.6 Dependency / configuration

- `.cargo/config.toml:1-12` commits a local path override (`paths = ["C:/Users/messi/projetos/mupdf-0.8.0-win"]`) and its own comments say "NÃO versionar". Also emits a PDB output-filename collision warning (lib vs bin). Remove from version control; add to `.gitignore`.
- `Cargo.lock` is gitignored (`.gitignore:3`) and not tracked. For a crate shipping a `[[bin]]`, this makes releases non-reproducible. Commit it (or document the policy).
- No `rust-version` (MSRV) declared in `Cargo.toml`. Add and test in CI.
- CodeQL (`.github/workflows/codeql.yml:29-33`) matrix only includes `javascript-typescript` and `python`; the Rust core is not SAST-scanned. Add `rust`.

---

## 3. FFI / bindings

### 3.1 API gaps

- **Node binding export gap** — `bindings/nodejs/index.js:215-221` exports `parseRaw`, `parsePdf`, `extractText`, `extractImages`, `version` but **not** `parseRawBytes`/`parsePdfBytes`/`extractImagesBytes`, even though FFI symbols and Python equivalents exist.
- **AGENTS.md claims bindings for Go and C# (.NET 8)** — present, but `bindings/go/` and `bindings/csharp/` have no README (only Python and Node do). README links them as "documentation" (`README.md:45`).
- **Version drift across artifacts** — current code is `0.3.0` (`Cargo.toml`, `pyproject.toml`, `package.json`, `csproj`), but `llms-full.txt` (root/docs/space_hf) still says `0.2.6` at lines 6/55/88, and `SECURITY.md:11-12` lists `0.2.x`/`< 0.2.0`.

### 3.2 Package metadata

- `bindings/python/rasgapdf/__init__.py` and `integrations/langchain.py`, `llama_index.py` exist; integrations use a standalone fallback when `langchain_core`/`llama_index` are absent — acceptable but untested.
- `bindings/csharp/` ships `rasgapdf.dll` locally and multiple `.nupkg` versions under `bin/Release/` and `dist/` (build artifacts; see §5).

### 3.3 FFI tests

No FFI tests exist for null pointers, invalid UTF-8, or `wasm_alloc`/`wasm_free` round-trips. Add them.

---

## 4. Documentation & web

### 4.1 MuPDF claims vs. pure-Rust reality

The project replaced MuPDF with a pure Rust engine (commit `f7b4f10`); `Cargo.lock` contains no `mupdf`. Docs still claim MuPDF linkage:
- `README.md:348-351` — "RasgaPDF links against MuPDF … derivative work … AGPL".
- `docs/ARCHITECTURE.md:56-57` — "Local DTOs Instead of `mupdf::page::StextPage`".
- `docs/SCHEMA.md:93` — rotation "reported by MuPDF".
- `docs/SCHEMA.md:202` — drawings "Mirrors PyMuPDF's `Page.get_drawings()`".
- `space_hf/README.md:56` — "MuPDF font metrics".
- `bindings/python/README.md:197`, `bindings/nodejs/README.md:191` — link-against-MuPDF statements.
- `space_hf/server.py:9` imports `fitz` (PyMuPDF) and `space_hf/requirements.txt:4` requires `PyMuPDF` (used only for pixmap conversion of embedded images) — contradicts "zero C/C++" positioning.

Also `README.md:313` — "Runs directly in your browser using client-side WebAssembly & PDF.js." Extraction is now pure Rust WASM; PDF.js is only used for rendering the canvas.
`space_hf/README.md:30-59` still documents a "Client-Side Fallback" via PDF.js that no longer matches the WASM-only path.

### 4.2 Stale file paths / scripts in AGENTS.md

- `AGENTS.md:15` references `src/graphics.rs` (actual: `src/drawing.rs` + `src/image.rs`).
- `AGENTS.md:16` references `src/schema.rs` (actual: `src/types.rs`).
- `AGENTS.md:21` describes `space_hf/` as "FastAPI native backend + PDF.js client-side fallback"; current code enforces pure Rust WASM extraction.
- `AGENTS.md:34,37` reference `./publish_all.sh` and `./sync_repos.sh`, which do not exist at repo root.
- `CONTRIBUTING.md:39` clone URL is the Hugging Face repo instead of GitHub.
- `CONTRIBUTING.md:55` references `python3 bindings/python/test_python.py` (does not exist; tests are in `tests/test_integrations.py`).
- `CONTRIBUTING.md:58` references `node bindings/nodejs/index.js` as a test entry point (not a test).

### 4.3 API/ID inconsistencies in LLM docs

`llms-full.txt` (root/docs/space_hf) shows `block_id: 0` and `table_id: 0` in JSON examples, but code starts both at `1` (`src/engine/doc.rs:578`, `src/table.rs` via `extract_tables(..., 1)`). `image_id`/`drawing_id` also start at `1`.

### 4.4 Broken / questionable links

- `docs/sitemap.xml` and `space_hf/sitemap.xml` reference `SCHEMA.html` and `ARCHITECTURE.html` (lines 20, 24), but only `.md` files exist. Broken for crawlers.
- `space_hf/robots.txt:38` points its `Sitemap:` to `https://martinsmessias.github.io/rasgapdf/sitemap.xml`, not the Hugging Face Space host.
- `README.md:45` links `bindings/go/` and `bindings/csharp/` as documentation though they have no README.
- External URLs (PyPI, crates.io, npm, HF, GitHub Pages) were not live-checked in this environment.

### 4.5 Accessibility (docs/index.html)

- No `favicon`/`apple-touch-icon`/web manifest (head, lines 4–26).
- Missing `og:image` (lines 16–20) and `twitter:image` (lines 22–25) despite `twitter:card=summary_large_image`.
- `dropzone` at line 173 is a `<div>` with click/drag handlers but no `role="button"`, `tabindex`, or keyboard handler.
- Tab buttons (~line 252) lack `role="tab"`, `aria-selected`, `aria-controls`.
- Canvas elements at lines 288 and 395 have no accessible name/role.
- Only one `aria-label` usage in the whole file; most icon-only controls rely on `title` only.

### 4.6 SEO

Head has good basics (title, meta description, canonical, robots, OG, Twitter, JSON-LD at lines 4–28). Missing: `og:image`/`twitter:image`, `theme-color`, favicon. Sitemap points to non-existent `.html` doc pages (see §4.4).

### 4.7 Web app issues (docs/index.html / space_hf)

- `parsePdfWithWasm` assumes `wasm_alloc`, `rasgapdf_parse_bytes_options`/`rasgapdf_parse_bytes`, `rasgapdf_parse_raw_bytes`, `rasgapdf_free_string`, and `memory` exports exist; failures surface only at runtime. No capability check beyond `rasgapdf_parse_bytes_options ?`.
- `loadPdfFile` throws "Pure Rust WebAssembly engine not loaded." when WASM fails, but the UI still attempts PDF.js rendering; error path handled, no progressive enhancement beyond that.
- `showLoadError` interpolates file name with a mojibake-prone dash.
- `space_hf/server.py` stores temp images under `mkdtemp` and cleans up in `finally`; OK, but returns 500 with generic message and logs only server-side.
- `space_hf/Dockerfile` installs system deps for the native lib but image is `python:3.11-slim`; if the shipped `.dll`/`.so` mismatches container arch, runtime load fails (not documented).

---

## 5. Repository hygiene & supply chain

### 5.1 Leaked / committed build artifacts

- **Historical native binaries** (8 MB each) committed in `6195ca8` → `25c1eca`, removed only in `67b5b19`; they remain in `.git`, inflating the repo (~42 MB `.git`). Paths: `bindings/nodejs/librasgapdf.so`, `bindings/python/rasgapdf/librasgapdf.so`, `bindings/nodejs/rasgapdf.dll`, `bindings/python/rasgapdf/rasgapdf.dll`, `space_hf/rasgapdf/librasgapdf.so`, `demo_gradio/librasgapdf.so`.
- **Current HEAD still tracks generated WASM binaries** — `docs/rasgapdf.wasm` and `space_hf/rasgapdf.wasm` (782,090 bytes each, identical hash).
- **Duplicated identical assets** — `docs/index.html` ↔ `space_hf/index.html` (69,778 bytes), `docs/sample.pdf` ↔ `space_hf/sample.pdf`, `docs/llms.txt`/`llms-full.txt` ↔ `space_hf/...`.
- **Local untracked artifacts** — `target/` (~6 GB, ignored), `bin/tmp/RasgaPdf.0.3.0.nupkg` (303,074 bytes), `.cargo/config.toml` (ignored via `.git/info/exclude` only).

### 5.2 .gitignore / .gitattributes

- `.gitignore` is missing `*.whl` (requested artifact class). Also missing `*.pdb`, `*.lib`, `*.a`, `*.rlib`, `*.rmeta`.
- `.cargo/config.toml` is not ignored by `.gitignore` (only via local exclude). Must never be committed.
- `.gitattributes` only marks `*.pdf`, `*.png`, `*.jpg`, `*.jpeg` as binary. Tracked `.wasm` (and other native artifacts) are exposed to line-ending normalization. Add `*.wasm binary` (and `.so`/`.dll`/`.dylib`/`.exe`/`.nupkg`/`.whl`/`.pdb`).

### 5.3 Secrets / tokens

**None found.** Pattern scan across tracked files and reachable Git objects for `ghp_`, `github_pat_`, `AKIA`, `hf_`, `npm_`, `pypi-`, `sk-`, `AIza` returned no hits. No `.env`, `.pypirc`, `.npmrc` (tracked), `*.pem`, or `*.key` tracked. Workflows reference secrets only via `${{ secrets.* }}`. Enable GitHub Secret Scanning + Push Protection (not verifiable from tree).

### 5.4 License consistency

Consistent — AGPL-3.0-or-later across `Cargo.toml`, `pyproject.toml`, `package.json`, `csproj`, `LICENSE`, `include/rasgapdf.h`, READMEs, docs, space_hf. No conflicting (MIT/Apache/BSD/ISC) declarations found.

### 5.5 SECURITY.md

- Supported-versions table (`SECURITY.md:5-13`) lists `0.2.x` / `< 0.2.0` but current version is `0.3.0`. Update to mark `0.3.x` supported.
- Claims cryptographic signing; pipeline provides PyPI attestations, npm `--provenance`, SBOM, but no cosign/Sigstore for crates.io/NuGet/wheels and no SLSA level declared. Align claims with actual guarantees.

---

## 6. CI/CD

### 6.1 CI gaps (`.github/workflows/ci.yml`)

- Rust build/test only on Ubuntu (`:30-31`); no Windows/macOS matrix.
- Node binding test (`:48-50`) runs `node -e "require('./bindings/nodejs/index.js')"` but CI never runs `npm install`, so `koffi` is unavailable → test likely fails. Add `npm ci`.
- Go test (`:57-60`) uses `go run cmd/main.go || true`, masking all failures. Remove `|| true`.
- No `cargo fmt --check`, `cargo clippy`, `cargo audit`, `npm audit`, `pip-audit`, or `govulncheck`.
- No dependency-review action on PRs.
- `Cargo.lock` gitignored → CI resolves latest compatible versions.

### 6.2 CodeQL (`.github/workflows/codeql.yml:23-28`)

Matrix only `javascript-typescript` and `python`. Rust core, C#, Go not scanned.

### 6.3 Dependabot (`.github/dependabot.yml`)

Covers Cargo, pip (`/bindings/python`), npm (`/bindings/nodejs`), GitHub Actions. Missing: Go modules (`/bindings/go`), NuGet (`bindings/csharp`), Docker (`space_hf/Dockerfile`). Groups all updates into single PRs (`groups: dependencies: patterns: ["*"]`), which can mask breakage.

### 6.4 Release pipeline (`.github/workflows/release.yml`)

- All native builds run on Ubuntu → only `librasgapdf.so` produced, but manifests declare Windows/macOS artifacts (`package.json` `files` includes `rasgapdf.dll`; `csproj` conditions; `pyproject.toml` package-data includes `*.dll`/`*.dylib`). Windows/macOS users get no native library.
- `actions/setup-dotnet@v6` (`:190`) and `NuGet/login@v1` (`:202`) are not pinned by commit SHA (unlike other actions). Supply-chain pinning gap.
- `anchore/sbom-action@...` version comment malformed (`v0.24.3.7.0.24.3`) at `:157`.
- `publish-crates` runs `cargo publish` without `--locked` and without tests/clippy in the release job (tests removed in commit `d05ee5c`).
- `publish-npm` does not run `npm install`/`npm test` before publish.
- Go binding is not published anywhere (no release job; tags `bindings/go/v*` exist but no workflow).
- Positive controls present (keep): PyPI OIDC trusted publishing + attestations (`:25-26,58`), npm `--provenance` (`:96`), NuGet OIDC Trusted Publishing with secret fallback (`:198-218`), SBOM attachment (`:154-167`), Scorecard workflow.

---

## 7. Test gaps

- No tests for binary XRef streams / object streams (`ObjStm`) or `/Prev` chains.
- No tests for malformed/truncated PDFs asserting graceful errors instead of panics (parser, lexer, flate).
- No tests for indirect `/Length`, predictor decoding edge cases, or empty streams.
- No FFI tests for null pointers, invalid UTF-8, or `wasm_alloc`/`wasm_free` round-trips.
- No tests for encrypted/password-protected PDFs (CLI only prints a hint).
- `tests/extraction.rs` covers only happy-path fixtures; no negative cases.
- Concurrency/Send+Sync not tested.
- Integration tests in `tests/test_integrations.py` exist but CI may not run them reliably.

---

## 8. Prioritized remediation checklist

### P0 — Correctness & memory safety
1. Fix XRef overwrite (`src/engine/xref.rs:195,267,277`) with `entry().or_insert()`.
2. Add recursion depth limit + visited set (`doc.rs:72-79,100-118`; `xref.rs:123-135`).
3. Bound decompression output (`flate.rs:42-48`).
4. Validate shift width (`xref.rs:293-297`).
5. Cap CID range expansion (`font.rs:40-43`).
6. Fix WASM allocator (`ffi.rs:266-279`).
7. Resolve indirect `/Length` (`parser.rs:166-167`).
8. Cap graphics-state stack (`content.rs:160-161`).
9. Fix `create_error_json` NUL panic (`ffi.rs:144-148`).

### P1 — Documentation accuracy
10. Remove/rewrite MuPDF linkage sections (README, ARCHITECTURE, SCHEMA, space_hf README, bindings READMEs).
11. Fix version in `llms-full.txt` (root/docs/space_hf) `0.2.6` → `0.3.0`; fix example IDs to start at 1.
12. Fix AGENTS.md file paths and script references; fix CONTRIBUTING.md clone URL and test commands; update SECURITY.md supported versions to `0.3.x`.
13. Correct `sitemap.xml` to point to existing URLs; fix Space robots sitemap host.
14. Update README web-visualizer claim (PDF.js rendering only).

### P2 — Repository hygiene
15. Purge historical binaries from Git history (filter-repo/BFG); force-push.
16. Decide policy for `docs/rasgapdf.wasm` / `space_hf/rasgapdf.wasm` (CI-generated vs committed); remove duplication.
17. Clean local `target/` (~6 GB), `bin/`, delete machine-specific `.cargo/config.toml`.
18. Add `*.whl` (and `*.pdb`/`*.lib`/`*.a`/`*.rlib`/`*.rmeta`) to `.gitignore`.
19. Add `*.wasm binary` (and other native types) to `.gitattributes`.
20. Commit `Cargo.lock` (or document policy); add `rust-version` MSRV.
21. Remove unused `rayon` dependency or implement parallel processing.

### P3 — CI / supply chain / tests
22. Add `npm ci` to Node CI job; remove `|| true` from Go test.
23. Expand CodeQL matrix or add Rust/C#/Go scanning; add `cargo audit`/`npm audit`/`pip-audit`.
24. Add missing Dependabot ecosystems (Go, NuGet, Docker).
25. Pin `actions/setup-dotnet` and `NuGet/login` by SHA.
26. Add cross-platform native builds (Windows/macOS) to release, or trim manifest claims.
27. Add Go binding publish job or document manual release.
28. Add malformed-input, FFI, XRef/object-stream, and predictor tests.
29. Add Diátaxis structure (`docs/tutorials`, `docs/how-to`, `docs/reference`, `docs/explanation`).
30. Add favicon, `og:image`, `twitter:image`, and ARIA/keyboard support to `docs/index.html`.
31. Document FFI safety contracts (`# Safety` docs on exported functions).
32. Add Node binding exports for `*Bytes` functions.
33. Reconcile `SECURITY.md` signing claims with pipeline guarantees.

---

## 9. Key paths

- Core engine: `src/engine/{content,doc,flate,font,lexer,object,parser,xref}.rs`
- FFI: `src/ffi.rs`
- Types: `src/types.rs` (not `src/schema.rs`)
- Drawing/image: `src/drawing.rs`, `src/image.rs` (not `src/graphics.rs`)
- Bindings: `bindings/{python,nodejs,go,csharp}/`
- Web: `docs/index.html`, `space_hf/`
- CI: `.github/workflows/{ci,codeql,release,scorecard}.yml`, `.github/dependabot.yml`
- Config: `Cargo.toml`, `.cargo/config.toml`, `.gitignore`, `.gitattributes`

*End of report.*
