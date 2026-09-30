// Screenplay pages rendered from the same layout engine as the PDF export
// (FSD §123: preview and print match the exported document). Positions come
// from Rust in inches; the page is drawn at true size and scaled for display.

import { useEffect } from "react";
import { createPortal } from "react-dom";
import type { InterchangePrintPage } from "../../ipc/generated/InterchangePrintPage";
import type { InterchangePrintPreview } from "../../ipc/generated/InterchangePrintPreview";

const PX_PER_IN = 96;
/** Courier 12pt ascent, so a line's baseline lands on its layout position. */
const ASCENT_IN = 0.125;

function Page({ page, widthIn, heightIn }: { page: InterchangePrintPage; widthIn: number; heightIn: number }) {
  return (
    <div
      className="ix-page"
      style={{ width: `${widthIn}in`, height: `${heightIn}in` }}
      aria-label={page.titlePage ? "Title page" : `Page ${page.number ?? ""}`}
      role="img"
    >
      {page.lines.map((l, i) => (
        <div key={i} className={`ix-line ix-${l.style}`} style={{ left: `${l.xIn}in`, top: `${l.yIn - ASCENT_IN}in` }}>
          {l.text}
        </div>
      ))}
    </div>
  );
}

/** One page, scaled to `scale` of true size. */
export function ScaledPage({ preview, index, scale }: { preview: InterchangePrintPreview; index: number; scale: number }) {
  const page = preview.pages[index];
  if (!page) return null;
  const w = preview.pageWidthIn * PX_PER_IN * scale;
  const h = preview.pageHeightIn * PX_PER_IN * scale;
  return (
    <div className="ix-page-frame" style={{ width: w, height: h }}>
      <div style={{ transform: `scale(${scale})`, transformOrigin: "top left" }}>
        <Page page={page} widthIn={preview.pageWidthIn} heightIn={preview.pageHeightIn} />
      </div>
    </div>
  );
}

export function pageTitle(preview: InterchangePrintPreview, index: number): string {
  const p = preview.pages[index];
  if (!p) return "";
  return p.titlePage ? "Title page" : `Page ${p.number ?? index + 1}`;
}

/**
 * Prints every page at true size through the system print dialog; `onDone`
 * runs when the dialog closes (`afterprint`). Only the pages are printed (see
 * `interchange.css`, `@media print`).
 */
export function PrintJob({ preview, onDone }: { preview: InterchangePrintPreview; onDone: () => void }) {
  useEffect(() => {
    document.body.classList.add("ix-printing");
    let finished = false;
    const finish = () => {
      if (finished) return;
      finished = true;
      document.body.classList.remove("ix-printing");
      onDone();
    };
    window.addEventListener("afterprint", finish);
    // Let the pages mount before the print dialog snapshots the document.
    const raf = window.requestAnimationFrame(() => window.print());
    return () => {
      window.cancelAnimationFrame(raf);
      window.removeEventListener("afterprint", finish);
      document.body.classList.remove("ix-printing");
    };
  }, [onDone]);

  return createPortal(
    <div className="ix-print-root" aria-hidden>
      {preview.pages.map((p, i) => (
        <Page key={i} page={p} widthIn={preview.pageWidthIn} heightIn={preview.pageHeightIn} />
      ))}
    </div>,
    document.body,
  );
}
