#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Gera PDFs de fixture para testar o RasgaPDF localmente.

Fixtures:
  texte_only.pdf   -> apenas texto            (baseline)
  image_only.pdf   -> apenas imagem           (bug: pagina vazia)
  mixed.pdf        -> texto + imagem + vetores (caso que hoje QUEBRA o texto)
  rotated.pdf      -> mixed com /Rotate 90

Nao depende de reportlab/pymupdf: monta o PDF a mao.
"""
import io
import os
import sys

from PIL import Image

OUT_DIR = os.path.dirname(os.path.abspath(__file__))


def jpeg_bytes(w=400, h=300):
    """JPEG RGB real com gradiente + borda, para nao ser uniforme."""
    img = Image.new("RGB", (w, h))
    px = img.load()
    for y in range(h):
        for x in range(w):
            px[x, y] = (x * 255 // w, y * 255 // h, 128)
    for x in range(w):
        px[x, 0] = (0, 0, 0)
        px[x, h - 1] = (0, 0, 0)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=85)
    return buf.getvalue()


def build_pdf(path, content_stream, images=(), rotate=None, text_present=True):
    """
    content_stream: bytes do /Contents
    images: lista de (nome, jpeg_bytes, width, height)
    rotate: int ou None ( /Rotate )
    """
    objects = {}  # num -> bytes do corpo do objeto

    n_content = 5
    n_font = 6
    n_img_first = 7

    # Recursos
    xobject_entries = []
    for i, (name, data, w, h) in enumerate(images):
        num = n_img_first + i
        xobject_entries.append("/%s %d 0 R" % (name, num))
        header = (
            b"<< /Type /XObject /Subtype /Image"
            b" /Width %d /Height %d /ColorSpace /DeviceRGB"
            b" /BitsPerComponent 8 /Filter /DCTDecode /Length %d >>\nstream\n"
            % (w, h, len(data))
        )
        objects[num] = header + data + b"\nendstream"

    resources = (
        b"<< /Font << /F1 %d 0 R >> /XObject << %s >> >>"
        % (n_font, " ".join(xobject_entries).encode("ascii"))
    )

    rotate_entry = b" /Rotate %d" % rotate if rotate else b""

    objects[1] = b"<< /Type /Catalog /Pages 2 0 R >>"
    objects[2] = b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"
    objects[3] = (
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]"
        + rotate_entry
        + b" /Resources "
        + resources
        + b" /Contents %d 0 R >>" % n_content
    )
    objects[n_content] = (
        b"<< /Length %d >>\nstream\n" % len(content_stream)
        + content_stream
        + b"\nendstream"
    )
    objects[n_font] = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"

    # Serializa
    max_num = max(objects)
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}
    for num in range(1, max_num + 1):
        if num not in objects:
            continue
        offsets[num] = len(out)
        out += b"%d 0 obj\n" % num + objects[num] + b"\nendobj\n"

    xref_pos = len(out)
    out += b"xref\n0 %d\n" % (max_num + 1)
    out += b"0000000000 65535 f \n"
    for num in range(1, max_num + 1):
        if num in offsets:
            out += b"%010d 00000 n \n" % offsets[num]
        else:
            out += b"0000000000 65535 f \n"
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (
        max_num + 1,
        xref_pos,
    )

    with open(path, "wb") as fh:
        fh.write(bytes(out))
    return path


TEXT = b"""
BT
/F1 24 Tf
72 700 Td
(Hello RasgaPDF) Tj
0 -36 Td
/F1 12 Tf
(Texto que hoje desaparece quando a pagina tem imagem.) Tj
0 -20 Td
(Segunda linha de conferencia espacial.) Tj
ET
"""

VECTORS = b"""
q
1 0 0 RG
4 w
100 500 m 500 500 l S
0 1 0 RG
2 w
150 560 m 250 640 350 480 450 560 c S
0 0 1 rg
60 60 200 110 re f
0.85 0 0.85 rg
300 80 180 90 re f
Q
"""

IMAGE_PLACEMENT = b"""
q
200 0 0 150 330 200 cm
/Im0 Do
Q
"""


def main():
    jpg = jpeg_bytes()

    build_pdf(
        os.path.join(OUT_DIR, "text_only.pdf"),
        TEXT,
    )

    build_pdf(
        os.path.join(OUT_DIR, "image_only.pdf"),
        IMAGE_PLACEMENT,
        images=[("Im0", jpg, 400, 300)],
    )

    build_pdf(
        os.path.join(OUT_DIR, "mixed.pdf"),
        TEXT + VECTORS + IMAGE_PLACEMENT,
        images=[("Im0", jpg, 400, 300)],
    )

    build_pdf(
        os.path.join(OUT_DIR, "rotated.pdf"),
        TEXT + VECTORS + IMAGE_PLACEMENT,
        images=[("Im0", jpg, 400, 300)],
        rotate=90,
    )

    print("fixtures gerados em", OUT_DIR)
    for name in ("text_only.pdf", "image_only.pdf", "mixed.pdf", "rotated.pdf"):
        p = os.path.join(OUT_DIR, name)
        print("  %-16s %6d bytes" % (name, os.path.getsize(p)))


if __name__ == "__main__":
    sys.exit(main())
