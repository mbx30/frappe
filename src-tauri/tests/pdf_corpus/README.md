# PDF corpus

This directory holds golden-file PDFs the preflight regression
suite loads at test time. The existing
`tests/preflight_regression.rs` and `tests/bleed_round_trip.rs`
tests early-return when the directory is missing, so the corpus
is strictly opt-in — copy real PDFs here to enable the full
sweep.

## Suggested files

For full coverage of the preflight engine, drop in:

* `business_card.pdf` — single-page CMYK print job with bleed and
  TrimBox.
* `book_cover.pdf` — large RGB image on a single page, used to
  exercise image resolution and color-space checks.
* `multi_page_report.pdf` — 5–20 page mixed-content document
  used to exercise page ops, OCG, and overprint.
* `damaged.pdf` — a hand-tweaked PDF that lopdf can open but
  that triggers specific findings (e.g. an ICCBased color space
  with a missing stream).
* `scanned_50mb.pdf` — a large scanned PDF, used to exercise
  compress_pdf and the optimization pass.

## Generating a corpus

Any PDF produced by common tools (Word, InDesign, LibreOffice,
`pdfme.sh`, etc.) is suitable. The tests do not modify the
corpus — they are read-only.

If you have a small `printpdf` script handy, this is enough to
produce a valid 1-page PDF for smoke-testing:

```rust
use printpdf::{PdfDocument, Mm};
let (doc, page1, layer1) = PdfDocument::new("hello", Mm(210.0), Mm(297.0), "l1");
doc.save(&mut std::io::BufWriter::new(std::fs::File::create("hello.pdf")?))?;
```

## Privacy

The corpus is committed via `.gitignore` (`*.pdf`) so no PDFs
ever end up in the repository. Tests that need them must be run
locally where the user has dropped the files in.
