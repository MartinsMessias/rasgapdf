"""RasgaPDF Adapter for PDF Parse Bench (phorn1/pdf-parse-bench).

This adapter registers RasgaPDF with the PDF Parse Bench framework,
converting PDF documents into layout-preserved Markdown with reconstructed
tables and exact reading order.
"""

from pathlib import Path
from typing import Dict, Any
import rasgapdf

try:
    from pdf_parse_bench.utilities import PDFParser
    from pdf_parse_bench.pipeline import run_cli
except ImportError:
    # Standalone mock base class when running outside the pdf-parse-bench repo
    class PDFParser:
        @classmethod
        def display_name(cls) -> str:
            return "RasgaPDF"

        def _write_output(self, content: str, output_path: Path) -> None:
            output_path.parent.mkdir(parents=True, exist_ok=True)
            with open(output_path, "w", encoding="utf-8") as f:
                f.write(content)

    def run_cli(parser_instance):
        import argparse
        parser = argparse.ArgumentParser(description=f"Run {parser_instance.display_name()}")
        parser.add_argument("-i", "--input", type=str, required=True, help="Input PDF directory or file")
        parser.add_argument("-o", "--output", type=str, default="results/rasgapdf", help="Output directory")
        args = parser.parse_args()

        in_p = Path(args.input)
        out_p = Path(args.output)
        out_p.mkdir(parents=True, exist_ok=True)

        files = [in_p] if in_p.is_file() else sorted(list(in_p.glob("**/*.pdf")))
        for f in files:
            target = out_p / f"{f.stem}.md"
            print(f"Parsing {f.name} -> {target.name}...")
            parser_instance.parse(f, target)


class RasgaPDFParser(PDFParser):
    """RasgaPDF official benchmark parser adapter."""

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

            # 1. Blocks
            for block in page.get("text_blocks", []):
                y0 = block.get("bbox", {}).get("y0", 0.0)
                lines = [line.get("text", "") for line in block.get("lines", []) if line.get("text")]
                text = "\n".join(lines).strip()
                if text:
                    elements.append((y0, text))

            # 2. Tables
            for table in page.get("tables", []):
                y0 = table.get("bbox", {}).get("y0", 0.0)
                table_md = self._format_table(table)
                if table_md:
                    elements.append((y0, table_md))

            # Sort in vertical reading order
            elements.sort(key=lambda x: x[0])
            page_text = "\n\n".join(el[1] for el in elements)
            if page_text:
                pages_md.append(page_text)

        markdown = "\n\n---\n\n".join(pages_md)
        self._write_output(markdown, output_path)
        return markdown


if __name__ == "__main__":
    run_cli(RasgaPDFParser())
