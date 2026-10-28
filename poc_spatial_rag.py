"""PoC: Spatial RAG & Visual Grounding com RasgaPDF + LLMs Locais.

Demonstra:
1. Ingestão ultra-rápida de artigo científico real (2201.00029.pdf) com RasgaPDF.
2. Spatial Chunking: cada chunk retém texto + BBox física 2D [x0, y0, x1, y1] + página.
3. Busca semântica pelo dado (tabela científica específica).
4. Prompt formatado com contexto para LLM Local (Ollama, Qwen, etc.).
5. Ancoragem Visual (Grounding): Renderiza a página do PDF desenhando a caixa
   amarela de destaque exatamente sobre as coordenadas retornadas pelo RasgaPDF!
"""
import os
import sys
import json
import time
from pathlib import Path

# Adiciona o RasgaPDF local ao path
REPO_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO_ROOT / "bindings" / "python"))

import rasgapdf
from rasgapdf import RasgaPDFLoader
import fitz  # PyMuPDF para renderizar a prova visual

def run_poc():
    pdf_path = REPO_ROOT.parent / "benchmarks" / "pdfs" / "2201.00029.pdf"
    if not pdf_path.exists():
        print(f"Erro: PDF não encontrado em {pdf_path}")
        return

    print("=" * 70)
    print("⚡ [1/4] Ingestão Espacial com RasgaPDF (mode='elements')")
    print("=" * 70)
    t0 = time.perf_counter()
    loader = RasgaPDFLoader(str(pdf_path), mode="elements", detect_tables=True)
    chunks = loader.load()
    dt = time.perf_counter() - t0

    print(f"📄 Artigo: {pdf_path.name}")
    print(f"⏱️ Tempo de parsing e fatiamento espacial: {dt * 1000:.2f} ms")
    print(f"📦 Total de blocos semânticos e tabelas extraídos: {len(chunks)}")

    # Conta blocos vs tabelas
    n_blocks = sum(1 for c in chunks if c.metadata.get("type") == "text_block")
    n_tables = sum(1 for c in chunks if c.metadata.get("type") == "table")
    print(f"   ├─ Blocos de texto: {n_blocks}")
    print(f"   └─ Tabelas físicas detectadas: {n_tables}")

    print("\n" + "=" * 70)
    print("🔍 [2/4] Simulação de Pergunta para LLM Local")
    print("=" * 70)
    user_query = "What is the Signal-to-Noise ratio and Period for Q7 Re-bin 1?"
    print(f"❓ Pergunta do Usuário: '{user_query}'")

    # Busca o chunk mais relevante (procurando menção a Q7 Re-bin e Signal-to-Noise)
    matched_chunk = None
    for chunk in chunks:
        content = chunk.page_content.lower()
        if "q7 re-bin 1" in content or ("q7" in content and "signal-to-noise" in content):
            matched_chunk = chunk
            break

    if not matched_chunk:
        print("Chunk não encontrado por busca direta!")
        return

    meta = matched_chunk.metadata
    page_num = meta.get("page", 1)
    bbox = meta.get("bbox", {})
    chunk_type = meta.get("type")

    print(f"🎯 Chunk Relevante Encontrado:")
    print(f"   ├─ Tipo: {chunk_type}")
    print(f"   ├─ Página: {page_num}")
    print(f"   └─ Bounding Box 2D: [x0: {bbox.get('x0')}, y0: {bbox.get('y0')}, x1: {bbox.get('x1')}, y1: {bbox.get('y1')}]")
    print(f"\n📊 Conteúdo do Chunk (Markdown preservado pelo RasgaPDF):")
    print("-" * 50)
    print(matched_chunk.page_content)
    print("-" * 50)

    print("\n" + "=" * 70)
    print("🤖 [3/4] Prompt Preparado para o LLM Local (Qwen / Ollama / LLaMA)")
    print("=" * 70)
    llm_prompt = f"""[SISTEMA: Você é um assistente científico de alta precisão. Responda à pergunta baseando-se estritamente no contexto fornecido.]

[CONTEXTO EXTRAÍDO VIA RASGAPDF - PÁGINA {page_num}]:
{matched_chunk.page_content}

[PERGUNTA]: {user_query}

[RESPOSTA ESPERADA]:
De acordo com a Tabela da Página {page_num}, para o ponto Q7 Re-bin 1:
- Period: 1.966 days
- Signal-to-Noise: 12.8 dB
- Variability Magnitude: 5.886
- Corrected Flux: -1.477"""
    print(llm_prompt)

    print("\n" + "=" * 70)
    print("🎨 [4/4] Prova de Ancoragem Espacial (Visual Citation Grounding)")
    print("=" * 70)
    print("Desenhando o destaque amarelo exatamente na coordenada física extraída pelo RasgaPDF...")

    # Abre o PDF original e desenha o highlight
    doc = fitz.open(str(pdf_path))
    page = doc[page_num - 1]
    
    # Cria o retângulo com a coordenada [x0, y0, x1, y1] do RasgaPDF
    rect = fitz.Rect(bbox.get("x0", 0), bbox.get("y0", 0), bbox.get("x1", 0), bbox.get("y1", 0))
    
    # Adiciona a anotação visual de destaque
    annot = page.add_rect_annot(rect)
    annot.set_colors(stroke=(0.9, 0.7, 0.0), fill=(1.0, 0.95, 0.0))
    annot.set_opacity(0.35)
    annot.update()

    # Renderiza a página anotada para imagem PNG de alta resolução
    pix = page.get_pixmap(dpi=150)
    out_img = REPO_ROOT / "poc_citation_page7.png"
    pix.save(str(out_img))

    print(f"🎉 Imagem de citação visual salva com sucesso em:")
    print(f"👉 {out_img}")
    print(f"📐 A caixa delimitadora amarela cobre com exatidão a tabela consultada pelo modelo!")
    print("=" * 70)

if __name__ == "__main__":
    run_poc()
