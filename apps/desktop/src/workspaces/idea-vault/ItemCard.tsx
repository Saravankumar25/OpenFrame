// One Vault item as a card (mocks 024/025): notes are text-first, images and
// sketches get thumbnails, files get file-type cues; unavailable external files
// keep their card with a red "Unavailable" badge (mock 036).

import { memo, useState, type KeyboardEvent, type MouseEvent } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Check, Link2, Mic, Music, Pin, Play, Quote as QuoteIcon, Video } from "lucide-react";
import { inTauri } from "../../ipc/client";
import type { AssetInfo } from "../../ipc/generated/AssetInfo";
import type { VaultItemDto } from "../../ipc/generated/VaultItemDto";
import type { VaultCollectionDto } from "../../ipc/generated/VaultCollectionDto";
import { extLabel, formatDuration, hostOf, isExternal, isUnavailable, isVisualType, metaLine } from "./model";

/** Local file URL through Tauri's restricted asset protocol (managed files only). */
export function assetUrl(a: AssetInfo | null | undefined): string | null {
  if (!a?.path || !a.available || !inTauri()) return null;
  try {
    return convertFileSrc(a.path);
  } catch {
    return null;
  }
}

export interface CardHandlers {
  onSelect: (item: VaultItemDto, e: MouseEvent | KeyboardEvent) => void;
  onToggle: (item: VaultItemDto) => void;
  onOpen: (item: VaultItemDto) => void;
}

function Thumb({ item, height }: { item: VaultItemDto; height: number }) {
  const [failed, setFailed] = useState(false);
  const url = assetUrl(item.asset);
  if (!url || failed) {
    return <div className={`photo ${item.itemType === "sketch" ? "sketch" : "cool"}`} style={{ width: "100%", height }} aria-hidden />;
  }
  return (
    <img
      src={url}
      alt=""
      loading="lazy"
      decoding="async"
      onError={() => setFailed(true)}
      style={{ width: "100%", height, objectFit: "cover", display: "block", background: "#e9e5db" }}
    />
  );
}

function FileCue({ item }: { item: VaultItemDto }) {
  const name = item.asset?.originalName;
  switch (item.itemType) {
    case "pdf":
      return <span className="fic pdf">PDF</span>;
    case "document":
      return <span className="fic doc">{extLabel(name)}</span>;
    case "audio":
      return <span className="fic aud"><Music size={16} /></span>;
    case "voice":
      return <span className="fic aud"><Mic size={16} /></span>;
    case "video":
      return <span className="fic vid"><Video size={16} /></span>;
    case "url":
      return <span className="fic url"><Link2 size={16} /></span>;
    default:
      return <span className="fic">{extLabel(name)}</span>;
  }
}

function CardBody({ item, full }: { item: VaultItemDto; full: boolean }) {
  const clamp = full ? "vv-clamp-12" : "vv-clamp-5";
  switch (item.itemType) {
    case "note":
      return (
        <div className="vb">
          {item.title && <div className="vt">{item.title}</div>}
          {item.body ? <div className={clamp}>{item.body}</div> : !item.title && <div className="muted">Untitled note</div>}
        </div>
      );
    case "quote":
      return (
        <div className="vb">
          <QuoteIcon size={13} aria-hidden style={{ color: "var(--purple)" }} />
          {item.title && <div className="vt">{item.title}</div>}
          <div className={clamp} style={{ fontStyle: "italic" }}>{item.body ? `“${item.body}”` : "Untitled quote"}</div>
          {item.sourceText && <div className="xs muted" style={{ marginTop: 4 }}>— {item.sourceText}</div>}
        </div>
      );
    case "url":
      return (
        <>
          <div className="filecard">
            <FileCue item={item} />
            <div style={{ minWidth: 0 }}>
              <div className="vt vv-ellipsis">{item.title ?? hostOf(item.url)}</div>
              <div className="xs muted vv-ellipsis">{hostOf(item.url)}</div>
            </div>
          </div>
          {item.body && <div className="vb"><div className={clamp}>{item.body}</div></div>}
        </>
      );
    default: {
      const visual = isVisualType(item.itemType);
      const dur = formatDuration(item.asset?.durationMs);
      return (
        <>
          {visual ? (
            <div className="rel">
              <Thumb item={item} height={full ? 170 : 110} />
            </div>
          ) : item.itemType === "video" ? (
            <div className="photo bus" style={{ width: "100%", height: full ? 120 : 70 }}>
              <span className="lb"><Play size={10} aria-hidden /> {dur ?? "Video"}</span>
            </div>
          ) : (
            <div className="filecard">
              <FileCue item={item} />
              <div style={{ minWidth: 0 }}>
                <div className="vt vv-ellipsis">{item.asset?.originalName ?? item.displayName}</div>
                {dur && <div className="xs muted">{dur}</div>}
              </div>
            </div>
          )}
          {(item.title || item.caption) && (
            <div className="vb">
              {item.title && <div className="vt">{item.title}</div>}
              {item.caption && <div className={clamp}>{item.caption}</div>}
            </div>
          )}
          {!item.title && !item.caption && (visual || item.itemType === "video") && (
            <div className="vb"><div className="vt vv-ellipsis">{item.displayName}</div></div>
          )}
        </>
      );
    }
  }
}

export const ItemCard = memo(function ItemCard({ item, selected, selectionMode, collections, full, handlers }: {
  item: VaultItemDto;
  selected: boolean;
  selectionMode: boolean;
  collections: VaultCollectionDto[];
  full: boolean;
  handlers: CardHandlers;
}) {
  const unavailable = isUnavailable(item);
  const cls = ["vcard", "vv-card", item.itemType === "note" ? "note" : "", item.itemType === "quote" ? "quote" : "", selected ? "sel" : ""]
    .filter(Boolean)
    .join(" ");
  return (
    <div
      className={cls}
      role="button"
      tabIndex={0}
      aria-pressed={selected}
      aria-label={`${item.displayName}${unavailable ? " (file unavailable)" : ""}`}
      data-item-id={item.id}
      onClick={(e) => handlers.onSelect(item, e)}
      onDoubleClick={() => handlers.onOpen(item)}
      onKeyDown={(e) => {
        if (e.target !== e.currentTarget) return;
        if (e.key === "Enter") {
          e.preventDefault();
          handlers.onOpen(item);
        } else if (e.key === " ") {
          e.preventDefault();
          handlers.onToggle(item);
        }
      }}
    >
      {unavailable ? (
        <span className="badge-un">Unavailable</span>
      ) : (
        <button
          type="button"
          className={`selc vv-check${selected ? " on" : ""}${selectionMode ? " show" : ""}`}
          aria-label={selected ? `Deselect ${item.displayName}` : `Select ${item.displayName}`}
          onClick={(e) => {
            e.stopPropagation();
            handlers.onToggle(item);
          }}
        >
          {selected && <Check size={12} />}
        </button>
      )}
      {item.pinned && (
        <span className="pinm" title="Pinned">
          <Pin size={12} aria-label="Pinned" />
        </span>
      )}
      <CardBody item={item} full={full} />
      <div className="meta">
        {isExternal(item) && <Link2 size={11} aria-label="Linked file outside the vault" />}
        <span className="vv-ellipsis">{metaLine(item, collections)}</span>
      </div>
      {item.matchReason && <div className="xs vv-match">{item.matchReason}</div>}
    </div>
  );
});
