"""Extract cloud-sec.pdf into page-tagged text for the RAG spike.

Writes data/cloud-sec.pages.txt with one marker per page:

    ===== PAGE <n> | thai=<ratio> =====

The ratio is the fraction of Thai characters on the page. Pages below the
quality threshold (0.30) are flagged LOW and are skipped by the Rust indexer,
so no answer can be grounded on garbled text.
"""

import os
import sys

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

from pypdf import PdfReader  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, ".."))
PDF = os.path.abspath(os.path.join(ROOT, "..", "..", "cloud-sec.pdf"))
OUT_DIR = os.path.join(ROOT, "data")
OUT = os.path.join(OUT_DIR, "cloud-sec.pages.txt")
LOW = 0.30


def thai_ratio(text):
    if not text:
        return 0.0
    thai = sum(1 for c in text if "\u0e00" <= c <= "\u0e7f")
    return thai / len(text)


def main():
    reader = PdfReader(PDF)
    os.makedirs(OUT_DIR, exist_ok=True)
    parts = []
    flagged = []
    for i, page in enumerate(reader.pages, 1):
        text = page.extract_text() or ""
        ratio = thai_ratio(text)
        parts.append("===== PAGE %d | thai=%.2f =====\n%s" % (i, ratio, text))
        if ratio < LOW:
            flagged.append(i)
        print(
            "page %2d  thai_ratio=%.2f  chars=%5d%s"
            % (i, ratio, len(text), "  LOW" if ratio < LOW else "")
        )
    with open(OUT, "w", encoding="utf-8") as fh:
        fh.write("\n".join(parts))
    print()
    print("wrote", OUT)
    print("low-quality pages (skipped by indexer):", flagged or "none")


if __name__ == "__main__":
    main()
