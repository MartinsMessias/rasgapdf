# ⚡ RasgaPdf (.NET)

<div align="center">

**The Blazing-Fast 2D Spatial PDF Parser powered by Pure Rust for .NET.**  
*Extract exact physical coordinates `[x0, y0, x1, y1]`, word spans, text blocks, tables, images & vector paths in milliseconds.*

[![NuGet version](https://img.shields.io/nuget/v/RasgaPdf.svg?color=blue&logo=nuget)](https://www.nuget.org/packages/RasgaPdf/)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Target .NET 8.0](https://img.shields.io/badge/.NET-8.0-purple.svg)](https://dotnet.microsoft.com/)

[**GitHub Repository**](https://github.com/MartinsMessias/rasgapdf) • [**Web Visualizer**](https://martinsmessias.github.io/rasgapdf/) • [**Live HF Demo**](https://huggingface.co/spaces/martinsmessias/rasgapdf)

</div>

---

## 🚀 Overview

Most .NET PDF libraries are heavy, complex, and guess document layout with fragile heuristics that break multi-column articles, financial statements, and invoices.

**RasgaPdf delivers uncompromising spatial precision at native hardware speed:**
- **Exact Bounding Boxes:** Every character, word span, line, block, and table has exact coordinates `[x0, y0, x1, y1]`.
- **Pure Native Speed:** Powered by a SIMD-parallelized pure Rust core. Parses pages in **~3 to 8 ms**.
- **Cross-Platform:** Bundled native runtimes for Windows (x64) and Linux (x64). Zero external C libraries or native tools required.
- **RAG & LLM Ready:** Directly feeds exact 2D visual layout into downstream embedding models, vector databases, and vision-language models.

---

## 📦 Installation

Install via NuGet Package Manager or .NET CLI:

```bash
dotnet add package RasgaPdf
```

---

## ⚡ Quickstart

### ⚡ Mode 1: Flat Physical Coordinates (Raw X/Y)
Fastest possible extraction (~3 to 8 ms per page). Returns an unnested list of 2D physical bounding boxes for every text span:

```csharp
using RasgaPdf;

// Extract raw coordinates
var raw = PdfParser.ParseRaw("document.pdf");
Console.WriteLine($"Extracted {raw.TotalSpans} spans across {raw.TotalPages} pages in {raw.ParseTimeMs:F2} ms");

foreach (var span in raw.Spans)
{
    Console.WriteLine($"[Page {span.Page}] ({span.X0:F2}, {span.Y0:F2}, {span.X1:F2}, {span.Y1:F2}) [{span.FontSize:F1}pt {span.FontName}] \"{span.Text}\"");
}
```

---

### 🎯 Mode 2: Hierarchical Structured Document
Reconstructs the visual hierarchy into visual paragraphs (`TextBlocks`), text lines (`Lines`), and auto-detects matrix tables:

```csharp
using RasgaPdf;

var options = new ParserOptions
{
    DetectTables = true,
    MinTableColumns = 3,
    ExtractImages = true,
    ExtractDrawings = true
};

var doc = PdfParser.ParsePdf("invoice.pdf", options);
Console.WriteLine($"Processed {doc.TotalPages} pages in {doc.ParseTimeMs:F2} ms");

foreach (var page in doc.Pages)
{
    Console.WriteLine($"\n--- Page {page.PageNumber} ({page.Width}x{page.Height} pt) ---");

    // 1. Text blocks & lines
    foreach (var block in page.TextBlocks)
    {
        Console.WriteLine($"Block #{block.BlockId} at ({block.Bbox.X0:F1}, {block.Bbox.Y0:F1}):");
        foreach (var line in block.Lines)
        {
            Console.WriteLine($"  Line: {line.Text}");
        }
    }

    // 2. Extracted matrix tables
    foreach (var table in page.Tables)
    {
        Console.WriteLine($"\nTable #{table.TableId} ({table.RowCount} rows x {table.ColCount} cols):");
        Console.WriteLine($"Headers: {string.Join(" | ", table.Headers)}");
        foreach (var row in table.Rows)
        {
            Console.WriteLine($"Row: {string.Join(" | ", row)}");
        }
    }
}
```

---

## 📄 License & Commercial Options

- **Open Source:** Licensed under the [GNU Affero General Public License v3.0 (AGPL-3.0)](https://www.gnu.org/licenses/agpl-3.0).
- **Commercial Exemption:** A commercial proprietary license is available for closed-source corporate platforms, private SaaS, and enterprise software without AGPL copyleft requirements. Contact: [messiasads2017@gmail.com](mailto:messiasads2017@gmail.com).
