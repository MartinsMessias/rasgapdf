"""Official LangChain Document Loader for RasgaPDF.

Ultra-fast 2D spatial PDF loading with exact bounding boxes, structured tables,
and zero-loss layout preservation for RAG pipelines.
"""
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterator, List, Optional, Union

try:
    from langchain_core.document_loaders import BaseLoader
    from langchain_core.documents import Document
except ImportError:
    # Standalone fallback when langchain_core is not installed
    class BaseLoader:
        def load(self) -> List["Document"]:
            return list(self.lazy_load())

    @dataclass
    class Document:
        page_content: str
        metadata: Dict[str, Any] = field(default_factory=dict)


class RasgaPDFLoader(BaseLoader):
    """LangChain Document Loader powered by RasgaPDF.

    Args:
        file_path_or_bytes: Path to PDF file or raw bytes of a PDF.
        mode: Extraction granularity:
            - "pages" (default): One Document per page with layout-preserved text & markdown tables.
            - "elements": One Document per spatial element (blocks and tables) with exact 2D bounding boxes.
            - "text": A single Document containing the entire normalized text stream.
        detect_tables: Whether to detect and reconstruct physical 2D tables. Default True.
        markdown_tables: Format tables as Markdown matrices inside page content. Default True.
    """

    def __init__(
        self,
        file_path_or_bytes: Union[str, Path, bytes],
        mode: str = "pages",
        detect_tables: bool = True,
        markdown_tables: bool = True,
    ):
        self.file_input = file_path_or_bytes
        self.mode = mode
        self.detect_tables = detect_tables
        self.markdown_tables = markdown_tables
        self.source = str(file_path_or_bytes) if not isinstance(file_path_or_bytes, (bytes, bytearray)) else "bytes_stream"

    def lazy_load(self) -> Iterator[Document]:
        import rasgapdf

        if self.mode == "text":
            text = rasgapdf.extract_text(self.file_input)
            yield Document(
                page_content=text,
                metadata={"source": self.source, "mode": "text"},
            )
            return

        doc = rasgapdf.parse_pdf(self.file_input, detect_tables=self.detect_tables)
        pages = doc.get("pages", [])

        if self.mode == "elements":
            for page in pages:
                page_num = page.get("page_number", 1)
                width = page.get("width", 0.0)
                height = page.get("height", 0.0)

                # 1. Text blocks
                for block in page.get("text_blocks", []):
                    lines_text = [line.get("text", "") for line in block.get("lines", []) if line.get("text")]
                    content = "\n".join(lines_text).strip()
                    if content:
                        yield Document(
                            page_content=content,
                            metadata={
                                "source": self.source,
                                "page": page_num,
                                "type": "text_block",
                                "bbox": block.get("bbox", {}),
                                "reading_order": block.get("reading_order", 0),
                                "page_width": width,
                                "page_height": height,
                            },
                        )

                # 2. Tables
                for t_idx, table in enumerate(page.get("tables", [])):
                    table_md = self._format_table_markdown(table) if self.markdown_tables else "\n".join(
                        ["\t".join([str(c) for c in row]) if isinstance(row, list) else str(row) for row in table.get("rows", [])]
                    )
                    if table_md.strip():
                        yield Document(
                            page_content=table_md,
                            metadata={
                                "source": self.source,
                                "page": page_num,
                                "type": "table",
                                "table_id": table.get("table_id", t_idx),
                                "bbox": table.get("bbox", {}),
                                "row_count": table.get("row_count", len(table.get("rows", []))),
                                "col_count": table.get("col_count", 0),
                                "page_width": width,
                                "page_height": height,
                            },
                        )
            return

        # Default mode: "pages"
        for page in pages:
            page_num = page.get("page_number", 1)
            parts = []

            # Add text blocks
            for block in page.get("text_blocks", []):
                lines_text = [line.get("text", "") for line in block.get("lines", []) if line.get("text")]
                text = "\n".join(lines_text).strip()
                if text:
                    parts.append(text)

            # Add formatted tables
            for table in page.get("tables", []):
                if self.markdown_tables:
                    table_str = self._format_table_markdown(table)
                else:
                    table_str = "\n".join(
                        ["\t".join([str(c) for c in row]) if isinstance(row, list) else str(row) for row in table.get("rows", [])]
                    )
                if table_str:
                    parts.append(table_str)

            page_content = "\n\n".join(parts).strip()
            yield Document(
                page_content=page_content,
                metadata={
                    "source": self.source,
                    "page": page_num,
                    "total_pages": len(pages),
                    "width": page.get("width", 0.0),
                    "height": page.get("height", 0.0),
                    "block_count": len(page.get("text_blocks", [])),
                    "table_count": len(page.get("tables", [])),
                },
            )

    def load(self) -> List[Document]:
        return list(self.lazy_load())

    @staticmethod
    def _format_table_markdown(table: Dict[str, Any]) -> str:
        rows = table.get("rows", [])
        headers = table.get("headers", [])
        if not rows and not headers:
            return ""

        matrix = []
        if headers and isinstance(headers, list):
            matrix.append([str(h).replace("\n", " ").strip() for h in headers])

        for row in rows:
            if isinstance(row, list):
                row_cells = [str(c).replace("\n", " ").strip() for c in row]
            elif isinstance(row, dict):
                row_cells = [str(c.get("text", "")).replace("\n", " ").strip() for c in row.get("cells", [])]
            else:
                row_cells = [str(row)]
            matrix.append(row_cells)

        if not matrix:
            return ""

        max_cols = max(len(r) for r in matrix)
        for r in matrix:
            while len(r) < max_cols:
                r.append("")

        md_lines = []
        header = matrix[0]
        md_lines.append("| " + " | ".join(header) + " |")
        md_lines.append("| " + " | ".join(["---"] * max_cols) + " |")
        for r in matrix[1:]:
            md_lines.append("| " + " | ".join(r) + " |")
        return "\n".join(md_lines)
