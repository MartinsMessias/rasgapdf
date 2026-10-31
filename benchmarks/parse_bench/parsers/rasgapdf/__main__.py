"""Official RasgaPDF Parser Entrypoint for pdf-parse-bench.

Usage inside phorn1/pdf-parse-bench:
    uv run -m parsers.rasgapdf -i data/2026-q1-tables-only -o results/rasgapdf
"""

import sys
from pathlib import Path
from typing import Dict, Any

try:
    import rasgapdf
except ImportError:
    raise ImportError(
        "RasgaPDF is required. Install via: pip install rasgapdf"
    )

from pdf_parse_bench.utilities import PDFParser
from pdf_parse_bench.pipeline import run_cli


class RasgaPDFParser(PDFParser):
    """RasgaPDF high-performance Rust parser integration for pdf-parse-bench."""

    @classmethod
    def display_name(cls) -> str:
        return "RasgaPDF"

    def _format_table(self, table: Dict[str, Any]) -> str:
        rows = table.get("rows", [])
        if not rows:
            return ""
        col_count = table.get("col_count", 0)
        if col_count == 0:
            col_count = max(len(r) if isinstance(r, list) else 1 for r in rows)

        md = []
        header = rows[0] if rows else []
        header_cells = [str(c).replace("\n", " ").strip() for c in header]
        while len(header_cells) < col_count:
            header_cells.append("")
        md.append("| " + " | ".join(header_cells) + " |")
        md.append("| " + " | ".join(["---"] * col_count) + " |")

        for row in rows[1:]:
            cells = [str(c).replace("\n", " ").strip() for c in row] if isinstance(row, list) else [str(row).strip()]
            while len(cells) < col_count:
                cells.append("")
            md.append("| " + " | ".join(cells) + " |")

        return "\n".join(md)

    def parse(self, pdf_path: Path, output_path: Path) -> str:
        """Parses a PDF into Markdown using native RasgaPDF engine."""
        doc = rasgapdf.parse_pdf(str(pdf_path), detect_tables=True)
        pages_md = []

        for page in doc.get("pages", []):
            elements = []

            # 1. Text blocks
            for block in page.get("text_blocks", []):
                y0 = block.get("bbox", {}).get("y0", 0.0)
                lines = [line.get("text", "") for line in block.get("lines", []) if line.get("text")]
                text = "\n".join(lines).strip()
                if text:
                    elements.append((y0, text))

            # 2. Reconstructed tables
            for table in page.get("tables", []):
                y0 = table.get("bbox", {}).get("y0", 0.0)
                table_md = self._format_table(table)
                if table_md:
                    elements.append((y0, table_md))

            # Sort in reading order (top to bottom)
            elements.sort(key=lambda x: x[0])
            page_text = "\n\n".join(el[1] for el in elements)
            if page_text:
                pages_md.append(page_text)

        markdown = "\n\n---\n\n".join(pages_md)
        self._write_output(markdown, output_path)
        return markdown


if __name__ == "__main__":
    run_cli(RasgaPDFParser())
