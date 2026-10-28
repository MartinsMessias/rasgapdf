"""Official LlamaIndex Reader for RasgaPDF.

High-throughput 2D spatial PDF reader for LlamaIndex indexers and agents.
"""
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

try:
    from llama_index.core.readers.base import BaseReader
    from llama_index.core.schema import Document
except ImportError:
    class BaseReader:
        pass

    @dataclass
    class Document:
        text: str
        extra_info: Dict[str, Any] = field(default_factory=dict)

        @property
        def metadata(self) -> Dict[str, Any]:
            return self.extra_info


class RasgaPDFReader(BaseReader):
    """LlamaIndex Reader powered by RasgaPDF.

    Args:
        mode: Granularity mode:
            - "pages" (default): Returns one Document per PDF page.
            - "elements": Returns one Document per text block or table with exact bounding boxes.
        detect_tables: Detect and extract physical tables. Default True.
        markdown_tables: Format tables as Markdown. Default True.
    """

    def __init__(
        self,
        mode: str = "pages",
        detect_tables: bool = True,
        markdown_tables: bool = True,
    ):
        self.mode = mode
        self.detect_tables = detect_tables
        self.markdown_tables = markdown_tables

    def load_data(
        self,
        file: Union[str, Path, bytes],
        extra_info: Optional[Dict[str, Any]] = None,
    ) -> List[Document]:
        from .langchain import RasgaPDFLoader

        loader = RasgaPDFLoader(
            file_path_or_bytes=file,
            mode=self.mode,
            detect_tables=self.detect_tables,
            markdown_tables=self.markdown_tables,
        )

        base_meta = extra_info or {}
        docs = []

        for doc in loader.lazy_load():
            meta = {**base_meta, **doc.metadata}
            try:
                # Use official llama_index Document if available
                from llama_index.core.schema import Document as LlamaDoc
                docs.append(LlamaDoc(text=doc.page_content, extra_info=meta))
            except ImportError:
                docs.append(Document(text=doc.page_content, extra_info=meta))

        return docs
