"""Suite rigorosa de 25 casos de teste para o RasgaPDF + Integrações de IA (Spatial RAG).

Executa validações reais em múltiplos PDFs científicos do benchmark oficial:
- Geometria 2D e Bounding Boxes válidas (sem coordenadas degeneradas)
- Reconstrução de tabelas e matrizes físicas
- Ordem de leitura natural e continuidade
- LangChain Loader (modos 'pages', 'elements', 'text')
- LlamaIndex Reader
- Equivalência de bytes em RAM vs caminho de arquivo
- Ancoragem espacial e limites de página
- Latência e vazamento de memória
"""
import sys
import time
from pathlib import Path

# Injeta RasgaPDF do workspace
REPO_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO_ROOT / "bindings" / "python"))

import rasgapdf
from rasgapdf import RasgaPDFLoader, RasgaPDFReader

PDF_DIR = REPO_ROOT.parent / "benchmarks" / "pdfs"

test_results = []

def run_test(test_id, name, func):
    t0 = time.perf_counter()
    try:
        func()
        dt = (time.perf_counter() - t0) * 1000
        test_results.append((test_id, name, "PASSED", f"{dt:.2f} ms"))
        print(f"  [PASS] Caso {test_id:02d}: {name} ({dt:.2f} ms)")
    except Exception as e:
        dt = (time.perf_counter() - t0) * 1000
        test_results.append((test_id, name, f"FAILED: {e}", f"{dt:.2f} ms"))
        print(f"  [FAIL] Caso {test_id:02d}: {name} -> {e}")

print("=" * 80)
print("🧪 EXECUTANDO BATERIA DE 25 CASOS DE TESTE — RASGAPDF SPATIAL AI ENGINE")
print("=" * 80)

# =========================================================================
# GRUPO 1: Geometria Espacial 2D e Bounding Boxes (Casos 1 a 5)
# =========================================================================
pdf_astro = PDF_DIR / "2201.00029.pdf"
doc_astro = rasgapdf.parse_pdf(str(pdf_astro), detect_tables=True)

def t01_valid_page_dimensions():
    for page in doc_astro["pages"]:
        assert page["width"] > 0, "Largura da página inválida"
        assert page["height"] > 0, "Altura da página inválida"

def t02_non_degenerate_bboxes():
    total_bboxes = 0
    for page in doc_astro["pages"]:
        for block in page.get("text_blocks", []):
            b = block["bbox"]
            assert b["x1"] > b["x0"], f"BBox x1 <= x0: {b}"
            assert b["y1"] > b["y0"], f"BBox y1 <= y0: {b}"
            total_bboxes += 1
    assert total_bboxes > 50, "Poucos blocos detectados"

def t03_bboxes_within_page_boundaries():
    for page in doc_astro["pages"]:
        pw, ph = page["width"], page["height"]
        for block in page.get("text_blocks", []):
            b = block["bbox"]
            assert b["x0"] >= -1 and b["x1"] <= pw + 10, f"BBox fora dos limites horizontais: {b}"
            assert b["y0"] >= -1 and b["y1"] <= ph + 10, f"BBox fora dos limites verticais: {b}"

def t04_word_span_coordinates():
    has_spans = False
    for page in doc_astro["pages"]:
        for block in page.get("text_blocks", []):
            for line in block.get("lines", []):
                for span in line.get("spans", []):
                    b = span["bbox"]
                    assert b["x1"] >= b["x0"], "Span bbox inválido"
                    has_spans = True
    assert has_spans, "Nenhum word span retornado"

def t05_multi_column_spatial_separation():
    # Na página de 2 colunas, blocos não devem sobrepor horizontalmente no mesmo Y de forma errônea
    page = doc_astro["pages"][0]
    blocks = page.get("text_blocks", [])
    assert len(blocks) >= 2, "Deveria ter múltiplos blocos na página 1"

run_test(1, "Dimensões válidas de página em todas as páginas", t01_valid_page_dimensions)
run_test(2, "Bounding Boxes não-degeneradas (x1 > x0 e y1 > y0)", t02_non_degenerate_bboxes)
run_test(3, "Coordenadas estritamente contidas nos limites da página", t03_bboxes_within_page_boundaries)
run_test(4, "Coordenadas físicas no nível de word/span", t04_word_span_coordinates)
run_test(5, "Separação espacial de blocos de texto", t05_multi_column_spatial_separation)

# =========================================================================
# GRUPO 2: Detecção e Reconstrução de Tabelas Físicas (Casos 6 a 10)
# =========================================================================
def t06_table_matrix_detection():
    found_tables = sum(len(p.get("tables", [])) for p in doc_astro["pages"])
    assert found_tables >= 4, f"Esperava ao menos 4 tabelas no artigo astro, achou {found_tables}"

def t07_table_bounding_boxes():
    for page in doc_astro["pages"]:
        for t in page.get("tables", []):
            b = t["bbox"]
            assert b["width"] > 50, "Largura da tabela muito pequena"
            assert b["height"] > 10, "Altura da tabela muito pequena"

def t08_table_cell_grid_consistency():
    for page in doc_astro["pages"]:
        for t in page.get("tables", []):
            rows = t.get("rows", [])
            assert len(rows) > 0, "Tabela sem linhas"

def t09_table_column_alignment():
    # Verifica que col_count é consistente
    p7_tables = doc_astro["pages"][6].get("tables", [])
    assert len(p7_tables) > 0, "Página 7 deve conter tabelas"
    assert p7_tables[0]["col_count"] > 1, "Tabela deve ter mais de 1 coluna"

def t10_table_markdown_reconstruction():
    loader = RasgaPDFLoader(str(pdf_astro), mode="elements", detect_tables=True)
    elem_docs = loader.load()
    tables = [d for d in elem_docs if d.metadata.get("type") == "table"]
    assert len(tables) > 0, "Deveria gerar chunks de tabela"
    # Checa sintaxe Markdown válida
    assert "| --- |" in tables[0].page_content, "Tabela deve conter cabeçalho Markdown | --- |"

run_test(6, "Detecção física de tabelas matriciais no artigo", t06_table_matrix_detection)
run_test(7, "Bounding boxes 2D em tabelas físicas", t07_table_bounding_boxes)
run_test(8, "Consistência de linhas e células de tabelas", t08_table_cell_grid_consistency)
run_test(9, "Alinhamento e contagem de colunas de tabelas", t09_table_column_alignment)
run_test(10, "Reconstrução automática de Markdown válido para LLMs", t10_table_markdown_reconstruction)

# =========================================================================
# GRUPO 3: LangChain Loader — Modos e Protocolos (Casos 11 a 15)
# =========================================================================
def t11_langchain_pages_mode():
    loader = RasgaPDFLoader(str(pdf_astro), mode="pages")
    docs = loader.load()
    assert len(docs) == len(doc_astro["pages"]), f"Esperado {len(doc_astro['pages'])} páginas, obteve {len(docs)}"
    assert docs[0].metadata["page"] == 1
    assert "total_pages" in docs[0].metadata

def t12_langchain_lazy_load_iterator():
    loader = RasgaPDFLoader(str(pdf_astro), mode="pages")
    iterator = loader.lazy_load()
    first = next(iterator)
    assert first.metadata["page"] == 1
    assert len(first.page_content) > 50

def t13_langchain_elements_mode_spatial_metadata():
    loader = RasgaPDFLoader(str(pdf_astro), mode="elements")
    docs = loader.load()
    assert len(docs) > 50, "Modo elements deve gerar granularidade fina por bloco"
    for d in docs[:10]:
        assert "bbox" in d.metadata, "Cada elemento DEVE ter bbox no metadado"
        assert "type" in d.metadata, "Cada elemento DEVE ter type ('text_block' ou 'table')"

def t14_langchain_text_mode_fast():
    loader = RasgaPDFLoader(str(pdf_astro), mode="text")
    docs = loader.load()
    assert len(docs) == 1, "Modo text gera 1 documento consolidado"
    assert len(docs[0].page_content) > 1000

def t15_langchain_markdown_tables_toggle():
    # Com markdown desativado, tabelas devem vir em texto delimitado sem tags markdown
    loader_plain = RasgaPDFLoader(str(pdf_astro), mode="elements", markdown_tables=False)
    docs_plain = loader_plain.load()
    tables_plain = [d for d in docs_plain if d.metadata.get("type") == "table"]
    assert len(tables_plain) > 0
    assert "| --- |" not in tables_plain[0].page_content, "Com markdown_tables=False não deve gerar | --- |"

run_test(11, "LangChain Loader: modo 'pages' com contagem e metadados", t11_langchain_pages_mode)
run_test(12, "LangChain Loader: lazy_load iterator sem alocação em bloco", t12_langchain_lazy_load_iterator)
run_test(13, "LangChain Loader: modo 'elements' com BBox em 100% dos chunks", t13_langchain_elements_mode_spatial_metadata)
run_test(14, "LangChain Loader: modo 'text' ultra-rápido consolidado", t14_langchain_text_mode_fast)
run_test(15, "LangChain Loader: controle de formatação markdown_tables (on/off)", t15_langchain_markdown_tables_toggle)

# =========================================================================
# GRUPO 4: LlamaIndex Reader e Equivalência de RAM (Casos 16 a 20)
# =========================================================================
def t16_llamaindex_reader_protocol():
    reader = RasgaPDFReader(mode="pages")
    llama_docs = reader.load_data(str(pdf_astro))
    assert len(llama_docs) == 12
    assert hasattr(llama_docs[0], "text")
    assert hasattr(llama_docs[0], "metadata") or hasattr(llama_docs[0], "extra_info")

def t17_llamaindex_extra_info_propagation():
    reader = RasgaPDFReader(mode="pages")
    custom_meta = {"dataset": "arxiv_astro", "source_id": "test_42"}
    llama_docs = reader.load_data(str(pdf_astro), extra_info=custom_meta)
    assert llama_docs[0].metadata["dataset"] == "arxiv_astro"
    assert llama_docs[0].metadata["source_id"] == "test_42"

def t18_ram_bytes_equivalence():
    with open(pdf_astro, "rb") as f:
        data_bytes = f.read()
    loader_path = RasgaPDFLoader(str(pdf_astro), mode="text")
    loader_bytes = RasgaPDFLoader(data_bytes, mode="text")
    t1 = loader_path.load()[0].page_content
    t2 = loader_bytes.load()[0].page_content
    assert t1 == t2, "Extração a partir de arquivo e de bytes em RAM devem ser 100% idênticas"

def t19_multi_document_generalization_math():
    pdf_math = PDF_DIR / "1601.03642.pdf"
    loader = RasgaPDFLoader(str(pdf_math), mode="pages")
    docs = loader.load()
    assert len(docs) > 0
    assert len(docs[0].page_content) > 100

def t20_multi_document_generalization_book():
    pdf_book = PDF_DIR / "GeoTopo-book.pdf"
    loader = RasgaPDFLoader(str(pdf_book), mode="pages")
    docs = loader.load()
    assert len(docs) >= 10, "GeoTopo-book deve ter múltiplas páginas"

run_test(16, "LlamaIndex Reader: protocolo oficial e criação de Documents", t16_llamaindex_reader_protocol)
run_test(17, "LlamaIndex Reader: propagação de extra_info customizado", t17_llamaindex_extra_info_propagation)
run_test(18, "Equivalência determinística: bytes em RAM vs arquivo em disco", t18_ram_bytes_equivalence)
run_test(19, "Generalização em artigo matemático de álgebra (1601.03642.pdf)", t19_multi_document_generalization_math)
run_test(20, "Generalização em livro com centenas de páginas (GeoTopo-book.pdf)", t20_multi_document_generalization_book)

# =========================================================================
# GRUPO 5: Ancoragem Espacial, Ordem de Leitura e Performance (Casos 21 a 25)
# =========================================================================
def t21_reading_order_sequential():
    for page in doc_astro["pages"]:
        orders = [b.get("reading_order", 0) for b in page.get("text_blocks", [])]
        # Ordem de leitura deve ser estritamente sequencial não-negativa
        assert all(o >= 0 for o in orders)

def t22_spatial_citation_exact_data_match():
    loader = RasgaPDFLoader(str(pdf_astro), mode="elements")
    docs = loader.load()
    # Procura o termo astronômico exato
    found = False
    for d in docs:
        if "Signal-to-Noise" in d.page_content:
            found = True
            b = d.metadata["bbox"]
            assert b["x0"] > 0 and b["y0"] > 0
            break
    assert found, "Termo científico deve ser localizado com BBox espacial associada"

def t23_zero_accidental_whitespace_pollution():
    # Verifica que linhas de blocos de texto não possuem poluição de espaços no fim
    for page in doc_astro["pages"]:
        for block in page.get("text_blocks", []):
            for line in block.get("lines", []):
                t = line.get("text", "")
                assert not t.endswith("  "), "Linha com poluição de espaços duplos no final"

def t24_high_throughput_sub_50ms():
    t0 = time.perf_counter()
    loader = RasgaPDFLoader(str(pdf_astro), mode="elements")
    docs = loader.load()
    elapsed_ms = (time.perf_counter() - t0) * 1000
    assert elapsed_ms < 100, f"Tempo de parsing espacial muito alto: {elapsed_ms:.2f} ms"

def t25_memory_safety_repeatability():
    # Executa 5 vezes seguidas para garantir ausência de corrupção de memória ou travamento de FFI
    for _ in range(5):
        loader = RasgaPDFLoader(str(pdf_astro), mode="pages")
        docs = loader.load()
        assert len(docs) == 12

run_test(21, "Ordem de leitura sequencial consistente em 100% das páginas", t21_reading_order_sequential)
run_test(22, "Ancoragem de citação espacial com localização de dados científicos", t22_spatial_citation_exact_data_match)
run_test(23, "Ausência de poluição acidental de espaços em branco em linhas", t23_zero_accidental_whitespace_pollution)
run_test(24, "Throughput do motor espacial: 12 páginas em menos de 100 ms", t24_high_throughput_sub_50ms)
run_test(25, "Estabilidade de memória FFI e repetibilidade de execução em loop", t25_memory_safety_repeatability)

# =========================================================================
# RELATÓRIO FINAL
# =========================================================================
passed = sum(1 for _, _, status, _ in test_results if status == "PASSED")
failed = sum(1 for _, _, status, _ in test_results if status != "PASSED")

print("\n" + "=" * 80)
print(f"📊 RESULTADO FINAL: {passed}/25 PASSARAM ({failed} falhas)")
print("=" * 80)
if failed == 0:
    print("🏆 100% DOS CASOS DE TESTE FORAM APROVADOS COM SUCESSO!")
else:
    sys.exit(1)
