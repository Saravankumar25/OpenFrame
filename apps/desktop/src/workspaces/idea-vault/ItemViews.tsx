// The four Vault views (FSD §5.5, mocks 024–027): Visual Grid, Card, List and
// Folder. Views change presentation only; all read the same item list. Large
// vaults stay smooth through row virtualization (TanStack Virtual).

import { useEffect, useMemo, useRef, useState, type ReactNode, type RefObject } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronRight, Folder, Link2, Pin } from "lucide-react";
import { ContextMenu, type MenuItemSpec } from "../../design-system";
import type { VaultCollectionDto } from "../../ipc/generated/VaultCollectionDto";
import type { VaultFolderDto } from "../../ipc/generated/VaultFolderDto";
import type { VaultItemDto } from "../../ipc/generated/VaultItemDto";
import { ItemCard, type CardHandlers } from "./ItemCard";
import { TYPE_LABEL, buildRows, isExternal, isUnavailable, relativeDay, type DisplayMode, type VaultRow } from "./model";

function useWidth(ref: RefObject<HTMLElement | null>): number {
  const [w, setW] = useState(900);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const cw = entries[0]?.contentRect.width;
      if (cw) setW(cw);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [ref]);
  return w;
}

export interface ViewProps {
  mode: DisplayMode;
  items: VaultItemDto[];
  selected: ReadonlySet<string>;
  collections: VaultCollectionDto[];
  handlers: CardHandlers;
  itemMenu: (item: VaultItemDto) => MenuItemSpec[];
  /** Folder view: sub-folders of the current folder. */
  folders?: VaultFolderDto[];
  onOpenFolder?: (f: VaultFolderDto) => void;
  folderMenu?: (f: VaultFolderDto) => MenuItemSpec[];
  splitPinned?: boolean;
  empty?: ReactNode;
}

export function ItemCanvas(props: ViewProps) {
  if (props.mode === "list") return <ListView {...props} />;
  return <CardCanvas {...props} />;
}

function CardCanvas({ mode, items, selected, collections, handlers, itemMenu, folders, onOpenFolder, folderMenu, splitPinned, empty }: ViewProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const width = useWidth(scrollRef);
  const minCol = mode === "card" ? 300 : 200;
  const columns = Math.max(1, Math.min(mode === "card" ? 3 : 6, Math.floor((width + 10) / (minCol + 10))));
  const rows: VaultRow[] = useMemo(
    () =>
      buildRows(items, columns, {
        splitPinned: splitPinned && mode === "grid",
        folders: mode === "folder" ? folders ?? [] : undefined,
      }),
    [items, columns, splitPinned, mode, folders],
  );
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (i) => (rows[i]?.kind === "header" ? 30 : rows[i]?.kind === "folders" ? 84 : mode === "card" ? 240 : 180),
    overscan: 6,
    getItemKey: (i) => rows[i]?.key ?? i,
  });
  const selectionMode = selected.size > 0;
  if (rows.length === 0) return <div className="vv-scroll" ref={scrollRef}>{empty}</div>;
  return (
    <div className="vv-scroll" ref={scrollRef} role="list" aria-label="Idea Vault items">
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((v) => {
          const row = rows[v.index];
          return (
            <div
              key={v.key}
              data-index={v.index}
              ref={virtualizer.measureElement}
              style={{ position: "absolute", top: 0, left: 0, right: 0, transform: `translateY(${v.start}px)` }}
            >
              {row.kind === "header" ? (
                <div className="h4" style={{ padding: "8px 0 4px" }}>{row.label}</div>
              ) : row.kind === "folders" ? (
                <div className="vv-row" style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}>
                  {row.folders.map((f) => (
                    <ContextMenu key={f.id} items={folderMenu?.(f) ?? []}>
                      <button type="button" className="card vv-folder" onDoubleClick={() => onOpenFolder?.(f)} onClick={() => onOpenFolder?.(f)}>
                        <Folder size={22} aria-hidden style={{ color: "var(--accent-d)" }} />
                        <span className="grow" style={{ textAlign: "left", minWidth: 0 }}>
                          <span className="b vv-ellipsis" style={{ display: "block" }}>{f.name}</span>
                          <span className="xs muted">{f.itemCount} {f.itemCount === 1 ? "item" : "items"}</span>
                        </span>
                      </button>
                    </ContextMenu>
                  ))}
                </div>
              ) : (
                <div className="vv-row" role="presentation" style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}>
                  {row.items.map((item) => (
                    <div role="listitem" key={item.id} style={{ minWidth: 0 }}>
                      <ContextMenu items={itemMenu(item)}>
                        <div>
                          <ItemCard
                            item={item}
                            selected={selected.has(item.id)}
                            selectionMode={selectionMode}
                            collections={collections}
                            full={mode === "card"}
                            handlers={handlers}
                          />
                        </div>
                      </ContextMenu>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ListView({ items, selected, collections, handlers, itemMenu, empty }: ViewProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 36,
    overscan: 12,
    getItemKey: (i) => items[i]?.id ?? i,
  });
  const collName = (id: string) => collections.find((c) => c.id === id)?.name;
  if (items.length === 0) return <div className="vv-scroll">{empty}</div>;
  return (
    <div className="vv-list" role="table" aria-label="Idea Vault items" aria-rowcount={items.length + 1}>
      <div className="vv-lrow vv-lhead" role="row">
        <span role="columnheader">Name</span>
        <span role="columnheader">Type</span>
        <span role="columnheader">Collection</span>
        <span role="columnheader">Tags</span>
        <span role="columnheader">Added</span>
        <span role="columnheader">Modified</span>
      </div>
      <div className="vv-scroll" ref={scrollRef} style={{ flex: 1 }}>
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((v) => {
            const item = items[v.index];
            const sel = selected.has(item.id);
            return (
              <ContextMenu key={v.key} items={itemMenu(item)}>
                <div
                  className={`vv-lrow${sel ? " sel" : ""}`}
                  role="row"
                  tabIndex={0}
                  aria-selected={sel}
                  data-item-id={item.id}
                  style={{ position: "absolute", top: 0, left: 0, right: 0, height: v.size, transform: `translateY(${v.start}px)` }}
                  onClick={(e) => handlers.onSelect(item, e)}
                  onDoubleClick={() => handlers.onOpen(item)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handlers.onOpen(item);
                    else if (e.key === " ") {
                      e.preventDefault();
                      handlers.onToggle(item);
                    }
                  }}
                >
                  <span role="cell" className="vv-ellipsis b" title={item.displayName}>
                    {item.pinned && <Pin size={11} aria-label="Pinned" style={{ marginRight: 4, color: "var(--accent-d)" }} />}
                    {isExternal(item) && <Link2 size={11} aria-label="Linked file" style={{ marginRight: 4 }} />}
                    {item.displayName}
                    {isUnavailable(item) && <span className="chip r" style={{ marginLeft: 6 }}>Unavailable</span>}
                  </span>
                  <span role="cell">{TYPE_LABEL[item.itemType]}</span>
                  <span role="cell" className="vv-ellipsis">{item.collectionIds.map(collName).filter(Boolean).join(", ") || "—"}</span>
                  <span role="cell" className="vv-ellipsis muted">{item.tags.join(" ") || ""}</span>
                  <span role="cell" className="muted">{relativeDay(item.createdAt)}</span>
                  <span role="cell" className="muted">{relativeDay(item.updatedAt)}</span>
                </div>
              </ContextMenu>
            );
          })}
        </div>
      </div>
    </div>
  );
}

/** Folder view breadcrumb (mock 027): "All items › Scenes to explore". */
export function FolderCrumbs({ path, onGo }: { path: VaultFolderDto[]; onGo: (id: string | null) => void }) {
  return (
    <nav className="row vv-crumbs" aria-label="Folder path">
      <button type="button" className="vv-link" onClick={() => onGo(null)}>All items</button>
      {path.map((f, i) => (
        <span key={f.id} className="row" style={{ gap: 4 }}>
          <ChevronRight size={13} aria-hidden className="muted" />
          {i === path.length - 1 ? (
            <b aria-current="page">{f.name}</b>
          ) : (
            <button type="button" className="vv-link" onClick={() => onGo(f.id)}>{f.name}</button>
          )}
        </span>
      ))}
    </nav>
  );
}
