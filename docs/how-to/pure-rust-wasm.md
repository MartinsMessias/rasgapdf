# How-to: Run Pure Rust RasgaPDF in the Browser with WebAssembly (WASM)

Because RasgaPDF is written in **100% pure Rust** with zero C/C++ dependencies and safe memory management, the entire PDF parsing engine can be compiled directly to WebAssembly (`wasm32-unknown-unknown`).

This recipe shows how to run RasgaPDF inside any modern web browser or client-side runtime with **zero server dependencies**, guaranteeing 100% data privacy and sub-second parsing on static hosting (GitHub Pages, Vercel, Netlify, Cloudflare Pages).

---

## Architecture: Zero-Server Client-Side Parsing

```
┌────────────────────────────────────────────────────────┐
│ Modern Web Browser (Client-Side Only)                  │
│                                                        │
│  User selects PDF (File / Blob / ArrayBuffer)          │
│               │                                        │
│               ▼                                        │
│  rasgapdf.wasm (Pure Rust ISO 32000 Engine)            │
│  ├── Allocates linear memory (wasm_alloc)              │
│  ├── Lexical & cross-reference parsing in Rust         │
│  ├── 2D spatial coordinate tracking                    │
│  └── Serializes Spatial JSON tree                      │
│               │                                        │
│               ▼                                        │
│  Web UI / Canvas Overlays / Visual Bounding Boxes     │
└────────────────────────────────────────────────────────┘
```

---

## 1. Getting the Pre-Compiled WebAssembly Binary

The pre-compiled WebAssembly binary is located in:
- `docs/rasgapdf.wasm` (used by GitHub Pages live visualizer)
- `space_hf/rasgapdf.wasm` (used by Hugging Face Space visualizer)

You can copy `rasgapdf.wasm` to your web project's public assets folder.

---

## 2. Browser Implementation (Vanilla JavaScript)

### Step 1: Initialize the WASM Engine
```javascript
let wasmEngine = null;

async function initRasgaPdfWasm(wasmUrl = 'rasgapdf.wasm') {
  const response = await fetch(wasmUrl);
  const wasmBytes = await response.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(wasmBytes);
  wasmEngine = instance.exports;
  console.log('⚡ RasgaPDF Pure Rust WASM Engine initialized!');
  return wasmEngine;
}
```

### Step 2: Parse PDF Bytes in WebAssembly
```javascript
function parsePdfBytes(pdfUint8Array, options = {}) {
  if (!wasmEngine) {
    throw new Error('WASM engine is not initialized yet. Call initRasgaPdfWasm() first.');
  }

  const startTime = performance.now();

  // 1. Allocate buffer in WASM memory and copy PDF bytes
  const pdfLength = pdfUint8Array.length;
  const pdfPtr = wasmEngine.wasm_alloc(pdfLength);
  const wasmMemory = new Uint8Array(wasmEngine.memory.buffer);
  wasmMemory.set(pdfUint8Array, pdfPtr);

  // 2. Prepare options JSON
  const defaultOptions = {
    detect_tables: true,
    min_table_columns: 3,
    column_tolerance: 16.0,
    row_tolerance: 6.0,
    extract_images: true,
    extract_drawings: true,
    filter_background_fills: true,
    ...options
  };

  const optsBytes = new TextEncoder().encode(JSON.stringify(defaultOptions) + '\0');
  const optsPtr = wasmEngine.wasm_alloc(optsBytes.length);
  const memOpts = new Uint8Array(wasmEngine.memory.buffer);
  memOpts.set(optsBytes, optsPtr);

  // 3. Invoke Pure Rust Parser
  const resPtr = wasmEngine.rasgapdf_parse_bytes_options
    ? wasmEngine.rasgapdf_parse_bytes_options(pdfPtr, pdfLength, optsPtr)
    : wasmEngine.rasgapdf_parse_bytes(pdfPtr, pdfLength);

  // 4. Read null-terminated JSON string output
  const memOut = new Uint8Array(wasmEngine.memory.buffer);
  let end = resPtr;
  while (memOut[end] !== 0) {
    end++;
  }
  const jsonStr = new TextDecoder('utf-8').decode(memOut.subarray(resPtr, end));
  const parsedDocument = JSON.parse(jsonStr);

  const durationMs = (performance.now() - startTime).toFixed(2);

  // 5. Free allocated strings and buffers to avoid memory leaks
  wasmEngine.rasgapdf_free_string(resPtr);
  wasmEngine.wasm_free(optsPtr, optsBytes.length);
  wasmEngine.wasm_free(pdfPtr, pdfLength);

  return {
    document: parsedDocument,
    parseTimeMs: durationMs
  };
}
```

### Step 3: Connect to File Input Element
```html
<!DOCTYPE html>
<html>
<head>
  <title>RasgaPDF WASM Demo</title>
</head>
<body>
  <h2>Upload PDF for Client-Side Spatial Parsing</h2>
  <input type="file" id="pdfFileInput" accept="application/pdf" />
  <pre id="outputLog"></pre>

  <script src="rasgapdf-wasm-wrapper.js"></script>
  <script>
    window.addEventListener('DOMContentLoaded', async () => {
      await initRasgaPdfWasm('rasgapdf.wasm');

      document.getElementById('pdfFileInput').addEventListener('change', async (e) => {
        const file = e.target.files[0];
        if (!file) return;

        const arrayBuffer = await file.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);

        const result = parsePdfBytes(uint8Array);
        document.getElementById('outputLog').textContent = 
          `Parsed in ${result.parseTimeMs} ms!\n` +
          `Pages: ${result.document.total_pages}\n` +
          JSON.stringify(result.document, null, 2);
      });
    });
  </script>
</body>
</html>
```

---

## 3. Building `rasgapdf.wasm` from Rust Source

If you want to build the WebAssembly artifact yourself:

1. Add the WebAssembly target to Rust:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. Build with release optimizations:
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```

3. (Optional) Optimize binary size with `wasm-opt`:
   ```bash
   wasm-opt -O3 -o rasgapdf_optimized.wasm target/wasm32-unknown-unknown/release/rasgapdf.wasm
   ```

---

## Advantages of Pure Rust WASM in Browser
- **Total Data Privacy**: Sensitive documents (bank statements, healthcare records, NDAs) never leave the user's device.
- **Zero Cloud Costs**: All compute is distributed to client devices.
- **Zero Setup**: Works directly on static web hosting or serverless edge CDNs without running Python or Docker backends.
