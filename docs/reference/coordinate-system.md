# Coordinate Contract & Geometry Reference

This specification defines the exact coordinate system contracts, normalization rules, and transformation behaviors enforced by RasgaPDF across all language bindings (Rust, Python, Node.js, Go, C#, and WebAssembly).

---

## 1. Units and Physical Metric
- **Units**: Typographical points ($1\text{ pt} = \frac{1}{72}\text{ inch} = 0.352778\text{ mm}$).
- **Precision**: 32-bit floating point (`f32` in Rust / C / Go, `float` in C#, `number` in Python / JavaScript).
- **Coordinate Tuple**: Every bounding box is expressed as `[x0, y0, x1, y1]`:
  - `x0`: Leftmost horizontal boundary.
  - `y0`: Topmost vertical boundary in top-down screen coordinates.
  - `x1`: Rightmost horizontal boundary ($x1 \ge x0$).
  - `y1`: Bottommost vertical boundary ($y1 \ge y0$).
  - `width` $= x1 - x0$.
  - `height` $= y1 - y0$.

---

## 2. Coordinate System: Top-Down vs Bottom-Up
PDF files natively define graphics in a Cartesian bottom-up system where $(0, 0)$ is at the bottom-left corner of the page MediaBox.

**RasgaPDF normalizes all coordinates to standard top-down screen space:**
- Origin $(0, 0)$ corresponds to the top-left corner of the page's visible crop box.
- $+X$ increases to the right.
- $+Y$ increases downwards towards the bottom of the page.
- This matches HTML `<canvas>`, PDF.js viewports, and modern UI rendering engines without requiring manual client-side inversion.

---

## 3. CropBox Precedence & Non-Zero Origin Normalization
Many scanned, printed, or cropped PDFs feature non-zero origins in `/MediaBox` or `/CropBox` (e.g., `[50, 100, 650, 900]`):

1. **Precedence**: If `/CropBox` is present in the page dictionary, it takes precedence over `/MediaBox`. If `/CropBox` is omitted, `/MediaBox` is used.
2. **Origin Offset Subtraction**:
   $$\begin{aligned}
   X_{\text{screen}} &= X_{\text{pdf}} - X_{\text{origin}} \\
   Y_{\text{screen}} &= H_{\text{visible}} - (Y_{\text{pdf}} - Y_{\text{origin}})
   \end{aligned}$$
   This guarantees that $x_0 \ge 0$ and $y_0 \ge 0$ relative to the visible page canvas, preventing citation highlight misalignment in PDF viewers.

---

## 4. Visual Rotation Handling (`ApplyRotation`)
PDF pages specify orientation via `/Rotate` ($0^\circ$, $90^\circ$, $180^\circ$, $270^\circ$).

- **Default (`ApplyRotation = false`)**: Coordinates are reported in unrotated physical page points. The page's native `/Rotate` integer is preserved in `page.rotation`.
- **Rotated Viewport (`ApplyRotation = true`)**: Coordinates and dimensions are projected through the clockwise rotation transform:
  - For $90^\circ$: Page width and height swap. Coordinates transform to match the user's on-screen orientation.
  - This allows highlight rectangles to be drawn directly over rotated scanned receipts or landscape tables without requiring manual matrix math in downstream clients.
