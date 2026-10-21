// Authored SVG icons, one stroke family (1.8px, round). Brand logos are filled marks.
import { raw } from './html.mjs';

const s = (d, extra = '') => raw(`<svg class="i" viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"${extra}>${d}</svg>`);

export const ICON = {
  apple: raw('<svg class="i logo" viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="currentColor"><path d="M15.53 3.83c.84-1.01 1.4-2.43 1.25-3.83-1.21.05-2.67.8-3.54 1.82-.78.9-1.45 2.34-1.27 3.71 1.34.1 2.72-.69 3.56-1.7zM12.15 6.9c-.95 0-2.42-1.08-3.96-1.04-2.04.03-3.91 1.18-4.96 3.01-2.12 3.68-.55 9.1 1.52 12.09 1.01 1.45 2.21 3.09 3.79 3.04 1.52-.07 2.09-.99 3.94-.99 1.83 0 2.35.99 3.96.95 1.64-.03 2.68-1.48 3.68-2.95 1.16-1.69 1.64-3.33 1.66-3.42-.04-.01-3.18-1.22-3.22-4.86-.03-3.04 2.48-4.49 2.6-4.56-1.43-2.09-3.62-2.32-4.39-2.38-2-.16-3.68 1.09-4.62 1.09z"/></svg>'),
  windows: raw('<svg class="i logo" viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="currentColor"><path d="M2 2h9.5v9.5H2zM12.5 2H22v9.5h-9.5zM2 12.5h9.5V22H2zM12.5 12.5H22V22h-9.5z"/></svg>'),
  check: s('<path d="M5 12.5l4.5 4.5L19 7.5"/>'),
  cross: s('<path d="M6 6l12 12M18 6L6 18"/>'),
  arrow: s('<path d="M5 12h14M13 6l6 6-6 6"/>'),
  chev: s('<path d="M6 9l6 6 6-6"/>'),
  menu: s('<path d="M3 6h18M3 12h18M3 18h18"/>'),
  play: s('<path d="M8 5.5v13l11-6.5z" fill="currentColor" stroke="none"/>'),
  pause: s('<path d="M8 5h3v14H8zM13 5h3v14h-3z" fill="currentColor" stroke="none"/>'),
  prev: s('<path d="M15 6l-6 6 6 6"/>'),
  next: s('<path d="M9 6l6 6-6 6"/>'),
  replay: s('<path d="M4 12a8 8 0 1 0 2.4-5.7M4 4v4.5h4.5"/>'),
  expand: s('<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>'),
  shrink: s('<path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5"/>'),
  clock: s('<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>'),
  level: s('<path d="M5 19v-5M12 19V9M19 19V5"/>'),
  info: s('<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5M12 8h.01"/>'),
  warn: s('<path d="M12 4l9 16H3z"/><path d="M12 10v4M12 17h.01"/>'),
  lock: s('<rect x="4.5" y="10.5" width="15" height="10" rx="2"/><path d="M8 10.5V7a4 4 0 0 1 8 0v3.5"/>'),
  laptop: s('<rect x="4" y="5" width="16" height="11" rx="1.5"/><path d="M2 19.5h20"/>'),
  folder: s('<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>'),
  offline: s('<path d="M3 3l18 18M8.5 16.5a5 5 0 0 1 7 0M5 13a10 10 0 0 1 5-2.7M14.5 10.4A10 10 0 0 1 19 13M2 9.5a15 15 0 0 1 4.5-2.6M12 20h.01"/>'),
  spark: s('<path d="M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5L18 18M6 18l2.5-2.5M15.5 8.5L18 6"/>'),
  doc: s('<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5M9 13h6M9 17h6"/>'),
  import: s('<path d="M12 3v12M7 10l5 5 5-5M4 19h16"/>'),
  palette: s('<path d="M12 3a9 9 0 1 0 0 18c1.4 0 2-1 2-2 0-1.5-1.2-1.8-1.2-3 0-1 .8-1.8 1.8-1.8H17a4 4 0 0 0 4-4C21 6.5 17 3 12 3z"/><circle cx="7.5" cy="11" r="1"/><circle cx="10" cy="7" r="1"/><circle cx="14.5" cy="7" r="1"/>'),
  key: s('<rect x="3" y="7" width="18" height="10" rx="2"/><path d="M7 11h.01M11 11h.01M15 11h.01M8 14h8"/>'),
  ext: s('<path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/>'),
};

// The OpenFrame frame mark, drawn with the brand gradient (#ofg defined once per page).
export const MARK = raw('<svg class="mk" viewBox="0 0 24 24" aria-hidden="true" focusable="false"><rect class="s" x="2" y="2" width="20" height="20" rx="2.2"/><path class="s" d="M7 2v20M17 2v20M2 12h20M2 7h5M2 17h5M22 7h-5M22 17h-5"/></svg>');
export const GRADIENT_DEFS = raw('<svg width="0" height="0" class="defs" aria-hidden="true" focusable="false"><defs><linearGradient id="ofg" gradientUnits="userSpaceOnUse" x1="2" y1="2" x2="22" y2="22"><stop offset="0" stop-color="#ff5d6c"/><stop offset=".55" stop-color="#ff9a3d"/><stop offset="1" stop-color="#ffc23d"/></linearGradient></defs></svg>');
