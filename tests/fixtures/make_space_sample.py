#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Gera o sample.pdf do Space (space_hf/sample.pdf).

O sample antigo nao tinha NENHUMA imagem, entao a aba de imagens e o overlay de
imagens nasciam vazios. Este gera uma pagina com texto + 2 imagens embutidas em
posicoes conhecidas + formas vetoriais.

Ground truth (coordenadas PDF, origem no canto inferior esquerdo):
  Im0  400x300 JPEG  ->  bbox (330, 320) a (530, 470)
  Im1  240x240 JPEG  ->  bbox ( 60, 320) a (180, 440)

Reaproveita o construtor de PDF "na mao" de tests/fixtures/make_fixtures.py.
"""
import io
import os
import sys

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import make_fixtures as mf  # noqa: E402

SPACE_DIR = os.path.normpath(os.path.join(HERE, "..", "..", "space_hf"))


def gradient_jpeg(w=400, h=300):
    """Gradiente diagonal com borda preta (visualmente obvio)."""
    img = Image.new("RGB", (w, h))
    px = img.load()
    for y in range(h):
        for x in range(w):
            px[x, y] = (x * 255 // w, y * 255 // h, 128)
    for x in range(w):
        px[x, 0] = (0, 0, 0)
        px[x, h - 1] = (0, 0, 0)
    for y in range(h):
        px[0, y] = (0, 0, 0)
        px[w - 1, y] = (0, 0, 0)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=85)
    return buf.getvalue()


def checker_jpeg(w=240, h=240, cell=24):
    """Xadrez amarelo/azul, bem distinto do gradiente."""
    img = Image.new("RGB", (w, h))
    px = img.load()
    for y in range(h):
        for x in range(w):
            if ((x // cell) + (y // cell)) % 2:
                px[x, y] = (250, 204, 21)
            else:
                px[x, y] = (30, 64, 175)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=85)
    return buf.getvalue()


CONTENT = b"""
BT
/F1 22 Tf
60 740 Td
(RasgaPDF Spatial Sample) Tj
0 -28 Td
/F1 11 Tf
(Lorem ipsum dolor sit amet, consectetur adipiscing elit. RasgaPDF) Tj
0 -16 Td
(reads every span, image and vector path and returns exact 2D) Tj
0 -16 Td
(coordinates, sed do eiusmod tempor incididunt ut labore et dolore) Tj
0 -16 Td
(magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation.) Tj
ET

q
0.20 0.35 0.90 rg
60 560 150 40 re f
0.95 0.75 0.20 rg
240 560 150 40 re f
0.15 0.65 0.40 rg
420 560 120 40 re f
Q

q
0.55 0.20 0.75 RG
3 w
60 520 m 560 520 l S
Q

q
200 0 0 150 330 320 cm
/Im0 Do
Q

q
120 0 0 120 60 320 cm
/Im1 Do
Q

BT
/F1 10 Tf
60 290 Td
(Lorem ipsum dolor sit amet, consectetur adipiscing elit. RasgaPDF marks) Tj
0 -14 Td
(each embedded image with a translucent orange bounding box.) Tj
ET

q
0.55 0.20 0.75 RG
0.8 w
60 121 480 124 re S
60 214 m 540 214 l S
60 183 m 540 183 l S
60 152 m 540 152 l S
180 121 m 180 245 l S
300 121 m 300 245 l S
420 121 m 420 245 l S
Q

BT
/F1 9 Tf
1 0 0 1 66 235 Tm (Item) Tj
1 0 0 1 186 235 Tm (X pt) Tj
1 0 0 1 306 235 Tm (Y pt) Tj
1 0 0 1 426 235 Tm (Type) Tj
1 0 0 1 66 204 Tm (Text block) Tj
1 0 0 1 186 204 Tm (60.0) Tj
1 0 0 1 306 204 Tm (740.0) Tj
1 0 0 1 426 204 Tm (text) Tj
1 0 0 1 66 173 Tm (Image Im0) Tj
1 0 0 1 186 173 Tm (330.0) Tj
1 0 0 1 306 173 Tm (322.0) Tj
1 0 0 1 426 173 Tm (image) Tj
1 0 0 1 66 142 Tm (Drawing 4) Tj
1 0 0 1 186 142 Tm (60.0) Tj
1 0 0 1 306 142 Tm (272.0) Tj
1 0 0 1 426 142 Tm (vector) Tj
ET
"""


def main():
    out = os.path.join(SPACE_DIR, "sample.pdf")
    mf.build_pdf(
        out,
        CONTENT,
        images=[
            ("Im0", gradient_jpeg(), 400, 300),
            ("Im1", checker_jpeg(), 240, 240),
        ],
    )
    print("gerado:", out, os.path.getsize(out), "bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
