#!/usr/bin/env python3
"""Generate a brief PDF explaining the Fiverr chat OCR + classification pipeline."""

from pathlib import Path

from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_JUSTIFY, TA_LEFT
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.platypus import (
    HRFlowable,
    KeepTogether,
    ListFlowable,
    ListItem,
    PageBreak,
    Paragraph,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)

OUT = Path(__file__).resolve().parent / "Conversation_Chat_OCR_Classification_Pipeline.pdf"

INK = colors.HexColor("#1a1a1a")
MUTED = colors.HexColor("#4a4a4a")
LINE = colors.HexColor("#d0d0d0")
ACCENT = colors.HexColor("#0b3d2e")
SOFT = colors.HexColor("#f3f6f4")
WHITE = colors.white


def styles():
    base = getSampleStyleSheet()
    s = {
        "cover_title": ParagraphStyle(
            "cover_title",
            parent=base["Title"],
            fontName="Helvetica-Bold",
            fontSize=26,
            leading=32,
            textColor=INK,
            alignment=TA_CENTER,
            spaceAfter=10,
        ),
        "cover_sub": ParagraphStyle(
            "cover_sub",
            parent=base["Normal"],
            fontName="Helvetica",
            fontSize=12,
            leading=16,
            textColor=MUTED,
            alignment=TA_CENTER,
            spaceAfter=6,
        ),
        "h1": ParagraphStyle(
            "h1",
            parent=base["Heading1"],
            fontName="Helvetica-Bold",
            fontSize=16,
            leading=20,
            textColor=ACCENT,
            spaceBefore=4,
            spaceAfter=10,
        ),
        "h2": ParagraphStyle(
            "h2",
            parent=base["Heading2"],
            fontName="Helvetica-Bold",
            fontSize=12,
            leading=16,
            textColor=INK,
            spaceBefore=12,
            spaceAfter=6,
        ),
        "body": ParagraphStyle(
            "body",
            parent=base["Normal"],
            fontName="Helvetica",
            fontSize=10,
            leading=14,
            textColor=INK,
            alignment=TA_JUSTIFY,
            spaceAfter=8,
        ),
        "bullet": ParagraphStyle(
            "bullet",
            parent=base["Normal"],
            fontName="Helvetica",
            fontSize=10,
            leading=14,
            textColor=INK,
            leftIndent=0,
        ),
        "mono": ParagraphStyle(
            "mono",
            parent=base["Code"],
            fontName="Courier",
            fontSize=8.5,
            leading=11,
            textColor=INK,
            backColor=SOFT,
            borderPadding=6,
        ),
        "caption": ParagraphStyle(
            "caption",
            parent=base["Normal"],
            fontName="Helvetica-Oblique",
            fontSize=8.5,
            leading=11,
            textColor=MUTED,
            alignment=TA_CENTER,
            spaceBefore=4,
            spaceAfter=10,
        ),
        "footer": ParagraphStyle(
            "footer",
            parent=base["Normal"],
            fontName="Helvetica",
            fontSize=8,
            textColor=MUTED,
            alignment=TA_CENTER,
        ),
        "stage_title": ParagraphStyle(
            "stage_title",
            parent=base["Normal"],
            fontName="Helvetica-Bold",
            fontSize=10,
            leading=13,
            textColor=ACCENT,
        ),
        "stage_body": ParagraphStyle(
            "stage_body",
            parent=base["Normal"],
            fontName="Helvetica",
            fontSize=9,
            leading=12,
            textColor=INK,
        ),
        "cell": ParagraphStyle(
            "cell",
            parent=base["Normal"],
            fontName="Helvetica",
            fontSize=9,
            leading=12,
            textColor=INK,
        ),
        "cell_head": ParagraphStyle(
            "cell_head",
            parent=base["Normal"],
            fontName="Helvetica-Bold",
            fontSize=9,
            leading=12,
            textColor=WHITE,
        ),
    }
    return s


def bullets(items, style):
    return ListFlowable(
        [ListItem(Paragraph(i, style), leftIndent=12, bulletColor=ACCENT) for i in items],
        bulletType="bullet",
        start="•",
        leftIndent=15,
        bulletFontSize=9,
        spaceBefore=2,
        spaceAfter=8,
    )


def stage_card(title, body, s):
    data = [[Paragraph(title, s["stage_title"])], [Paragraph(body, s["stage_body"])]]
    t = Table(data, colWidths=[6.5 * inch])
    t.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, -1), SOFT),
                ("BOX", (0, 0), (-1, -1), 0.5, LINE),
                ("LEFTPADDING", (0, 0), (-1, -1), 10),
                ("RIGHTPADDING", (0, 0), (-1, -1), 10),
                ("TOPPADDING", (0, 0), (0, 0), 8),
                ("BOTTOMPADDING", (0, -1), (-1, -1), 8),
                ("TOPPADDING", (0, 1), (-1, 1), 2),
            ]
        )
    )
    return KeepTogether([t, Spacer(1, 8)])


def add_footer(canvas, doc):
    canvas.saveState()
    canvas.setStrokeColor(LINE)
    canvas.setLineWidth(0.5)
    canvas.line(0.75 * inch, 0.55 * inch, letter[0] - 0.75 * inch, 0.55 * inch)
    canvas.setFont("Helvetica", 8)
    canvas.setFillColor(MUTED)
    canvas.drawString(0.75 * inch, 0.35 * inch, "Conversation Chat OCR & Classification Pipeline")
    canvas.drawRightString(letter[0] - 0.75 * inch, 0.35 * inch, f"Page {doc.page}")
    canvas.restoreState()


def build():
    s = styles()
    doc = SimpleDocTemplate(
        str(OUT),
        pagesize=letter,
        leftMargin=0.75 * inch,
        rightMargin=0.75 * inch,
        topMargin=0.7 * inch,
        bottomMargin=0.75 * inch,
        title="Conversation Chat OCR & Classification Pipeline",
        author="ARAFAT",
        subject="Problem, tech stack, and end-to-end pipeline",
    )
    story = []

    # Cover
    story.append(Spacer(1, 1.6 * inch))
    story.append(Paragraph("Conversation Chat OCR &amp; Classification", s["cover_title"]))
    story.append(Paragraph("Problem Brief, Tech Stack, and End-to-End Pipeline", s["cover_sub"]))
    story.append(Spacer(1, 0.25 * inch))
    story.append(HRFlowable(width="60%", thickness=1, color=ACCENT, spaceBefore=4, spaceAfter=12, hAlign="CENTER"))
    story.append(
        Paragraph(
            "How to extract conversation text from image-based inbox PDFs/PNGs,<br/>"
            "structure it into messages, and classify intent with a lean local pipeline.",
            s["cover_sub"],
        )
    )
    story.append(Spacer(1, 0.8 * inch))
    story.append(Paragraph("Stack: Rust · Tesseract · TensorFlow", s["cover_sub"]))
    story.append(Paragraph("Document purpose: design overview (no implementation code)", s["cover_sub"]))
    story.append(PageBreak())

    # 1. Problem
    story.append(Paragraph("1. The Problem", s["h1"]))
    story.append(
        Paragraph(
            "The workspace contains Fiverr Inbox exports: large PDFs (some over 100&nbsp;MB) "
            "and at least one PNG screenshot. These files mostly capture <b>conversation UI as images</b> "
            "inside PDF pages—not selectable digital text. Standard PDF text extraction "
            "(copy/paste, pdftotext, naive parsers) fails or returns empty/garbage output.",
            s["body"],
        )
    )
    story.append(Paragraph("What we need", s["h2"]))
    story.append(
        bullets(
            [
                "<b>Extract</b> readable text from image-based PDFs and PNGs.",
                "<b>Structure</b> that text into chat messages (speaker, time, body) instead of a raw OCR soup.",
                "<b>Classify</b> messages by intent (e.g. revision, delivery, price, dispute), then roll up labels for each conversation/file.",
            ],
            s["bullet"],
        )
    )
    story.append(Paragraph("Why this is hard", s["h2"]))
    story.append(
        bullets(
            [
                "Pages are screenshots: OCR must recover characters from pixels.",
                "Chat UI adds noise: Online status, buttons, badges, sidebars, system notices.",
                "Speaker and timestamps are layout cues, not plain paragraphs.",
                "Long exports make naive high-DPI OCR of every page expensive in CPU, RAM, and time.",
            ],
            s["bullet"],
        )
    )
    story.append(PageBreak())

    # 2. Tech stack
    story.append(Paragraph("2. Tech Stack", s["h1"]))
    story.append(
        Paragraph(
            "The chosen stack splits work by strength: Rust for fast local extraction, "
            "Tesseract for character recognition, TensorFlow for text intent classification.",
            s["body"],
        )
    )

    header = [
        Paragraph("Layer", s["cell_head"]),
        Paragraph("Tool", s["cell_head"]),
        Paragraph("Responsibility", s["cell_head"]),
    ]
    rows = [
        header,
        [
            Paragraph("Orchestration &amp; I/O", s["cell"]),
            Paragraph("Rust", s["cell"]),
            Paragraph("Render PDF/PNG pages, preprocess images, call OCR, group lines into messages, cache JSON, batch files.", s["cell"]),
        ],
        [
            Paragraph("Character recognition", s["cell"]),
            Paragraph("Tesseract", s["cell"]),
            Paragraph("Turn page images into characters, words, and lines (with bounding boxes when needed).", s["cell"]),
        ],
        [
            Paragraph("Classification", s["cell"]),
            Paragraph("TensorFlow", s["cell"]),
            Paragraph("Train/serve a small text model that maps message text → intent label + confidence.", s["cell"]),
        ],
        [
            Paragraph("Inference bridge", s["cell"]),
            Paragraph("Python TF or ONNX", s["cell"]),
            Paragraph("Train in Python/TF; serve via a small Python process or export ONNX and run from Rust (ort).", s["cell"]),
        ],
    ]
    table = Table(rows, colWidths=[1.4 * inch, 1.3 * inch, 3.8 * inch])
    table.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, 0), ACCENT),
                ("BACKGROUND", (0, 1), (-1, 1), SOFT),
                ("BACKGROUND", (0, 3), (-1, 3), SOFT),
                ("TEXTCOLOR", (0, 0), (-1, 0), WHITE),
                ("GRID", (0, 0), (-1, -1), 0.4, LINE),
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("LEFTPADDING", (0, 0), (-1, -1), 6),
                ("RIGHTPADDING", (0, 0), (-1, -1), 6),
                ("TOPPADDING", (0, 0), (-1, -1), 6),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 6),
            ]
        )
    )
    story.append(table)
    story.append(Paragraph("Table 1. Stack roles across the pipeline.", s["caption"]))

    story.append(Paragraph("What we deliberately avoid by default", s["h2"]))
    story.append(
        bullets(
            [
                "Vision LLMs on every page (accurate for layout, but costly and privacy-sensitive).",
                "Running TensorFlow natively inside Rust for training (weak ergonomics); prefer Python train → ONNX/Python serve.",
                "Re-OCRing the same file on every experiment—always cache structured JSON.",
            ],
            s["bullet"],
        )
    )
    story.append(PageBreak())

    # 3. Pipeline overview
    story.append(Paragraph("3. Entire Pipeline", s["h1"]))
    story.append(
        Paragraph(
            "Data flows from pixels to labels. Tesseract only recognizes characters. "
            "Rust turns OCR output into conversation structure. TensorFlow classifies cleaned message text.",
            s["body"],
        )
    )

    flow = (
        "<b>PDF/PNG</b> → page images → preprocess → <b>Tesseract OCR</b> → raw lines → "
        "<b>Rust message structuring</b> → cached JSON → <b>TensorFlow intents</b> → conversation rollup"
    )
    story.append(Paragraph(flow, s["body"]))
    story.append(Paragraph("Figure 1. High-level data path from file to labels.", s["caption"]))

    story.append(Paragraph("Stage-by-stage detail", s["h2"]))

    story.append(
        stage_card(
            "Stage 1 — File to page images (Rust)",
            "Open each PDF or PNG. Render PDF pages at 150–200 DPI (or load the PNG). "
            "Process page-by-page so large 100&nbsp;MB exports do not stay fully decoded in memory.",
            s,
        )
    )
    story.append(
        stage_card(
            "Stage 2 — Preprocess (Rust)",
            "Optional grayscale, contrast, and crop to the chat pane. "
            "Improves OCR accuracy and reduces work on sidebars and chrome.",
            s,
        )
    )
    story.append(
        stage_card(
            "Stage 3 — Character recognition (Tesseract)",
            "OCR emits characters → words → lines, ideally with bounding boxes. "
            "Output is still messy UI text (status labels, timestamps, mixed speakers).",
            s,
        )
    )
    story.append(
        stage_card(
            "Stage 4 — Lines to structured messages (Rust)",
            "Sort by reading order. Drop chrome via rules. Group lines into messages using gaps, "
            "timestamps, name patterns, and left/right layout cues. Emit speaker, time, and text. Cache JSON per file.",
            s,
        )
    )
    story.append(
        stage_card(
            "Stage 5 — Classification inputs",
            "Each message text (optionally with a short neighbor context) becomes one classifier example. "
            "Obvious Fiverr system notices can be rule-labeled and skip the model.",
            s,
        )
    )
    story.append(
        stage_card(
            "Stage 6 — Intent classification (TensorFlow)",
            "A small text model predicts intent + confidence, e.g. greeting, scope, price, revision, "
            "delivery, dispute, system, other.",
            s,
        )
    )
    story.append(
        stage_card(
            "Stage 7 — Conversation rollup",
            "Aggregate message labels per file/thread: intent histogram, risk flag if dispute/refund language appears, "
            "and fields useful for search or review. Final artifact is JSON + labels—not page bitmaps.",
            s,
        )
    )
    story.append(PageBreak())

    # 4. Example transforms
    story.append(Paragraph("4. Example Data Shapes", s["h1"]))
    story.append(Paragraph("After OCR (raw lines — illustrative)", s["h2"]))
    story.append(
        Paragraph(
            "Online<br/>BuyerName<br/>Hi, can you revise the logo colors?<br/>11:42 AM<br/>"
            "You<br/>Sure, I will update and deliver today.<br/>11:45 AM<br/>"
            "Delivery<br/>Your order was delivered",
            s["mono"],
        )
    )
    story.append(Spacer(1, 8))
    story.append(Paragraph("After structuring (messages JSON — illustrative)", s["h2"]))
    story.append(
        Paragraph(
            '{ "speaker": "buyer", "time": "11:42 AM", '
            '"text": "Hi, can you revise the logo colors?" }<br/>'
            '{ "speaker": "seller", "time": "11:45 AM", '
            '"text": "Sure, I will update and deliver today." }<br/>'
            '{ "speaker": "system", "time": null, '
            '"text": "Your order was delivered" }',
            s["mono"],
        )
    )
    story.append(Spacer(1, 8))
    story.append(Paragraph("After classification (labels — illustrative)", s["h2"]))
    story.append(
        Paragraph(
            '{ "text": "Hi, can you revise the logo colors?", "intent": "revision", "confidence": 0.91 }<br/>'
            '{ "text": "Sure, I will update and deliver today.", "intent": "delivery", "confidence": 0.84 }<br/>'
            '{ "text": "Your order was delivered", "intent": "system", "confidence": 1.0 }',
            s["mono"],
        )
    )
    story.append(PageBreak())

    # 5. Resources
    story.append(Paragraph("5. Resource Profile", s["h1"]))
    story.append(
        Paragraph(
            "The architecture is not inherently too heavy if OCR is cached and DPI stays moderate. "
            "Cost is dominated by rasterization and Tesseract—not by classifying short message strings.",
            s["body"],
        )
    )
    story.append(
        bullets(
            [
                "<b>First run</b> on a large PDF: minutes-scale (OCR dominates).",
                "<b>Later runs</b>: fast when structured JSON cache hits.",
                "<b>TF inference</b>: cheap relative to OCR; CPU is enough for a small text model.",
                "<b>GPU</b>: optional for training only; not required for daily extraction/inference.",
                "<b>Guards</b>: 150–200 DPI, page batches, delete temp images after OCR, skip blank/duplicate pages.",
            ],
            s["bullet"],
        )
    )

    story.append(Paragraph("6. Bottom Line", s["h1"]))
    story.append(
        Paragraph(
            "Treat Fiverr inbox PDFs as images. Use <b>Rust + Tesseract</b> to turn pixels into structured chat messages, "
            "cache that output, then use a <b>small TensorFlow text classifier</b> for message intents and a conversation-level rollup. "
            "Keep vision LLMs and high-DPI full-file OCR out of the default path so the system stays practical on a normal machine.",
            s["body"],
        )
    )
    story.append(Spacer(1, 16))
    story.append(HRFlowable(width="100%", thickness=0.5, color=LINE, spaceBefore=4, spaceAfter=10))
    story.append(
        Paragraph(
            "Source context: local Chat folder Fiverr Inbox PDF/PNG exports. "
            "This document describes the agreed design approach for discussion and planning.",
            s["caption"],
        )
    )

    doc.build(story, onFirstPage=add_footer, onLaterPages=add_footer)
    print(OUT)


if __name__ == "__main__":
    build()
