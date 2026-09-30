// Vault left panel (mock 024, 178px): smart views, collections, folders, tags.

import type { ReactNode } from "react";
import { Clock, Folder, Hash, LayoutGrid, Layers, Pencil, Pin, Plus } from "lucide-react";
import { ContextMenu, type MenuItemSpec } from "../../design-system";
import type { VaultCollectionDto } from "../../ipc/generated/VaultCollectionDto";
import type { VaultFolderDto } from "../../ipc/generated/VaultFolderDto";
import type { VaultOverview } from "../../ipc/generated/VaultOverview";
import type { VaultView } from "../../ipc/generated/VaultView";
import { folderTree, sameView } from "./model";

function Entry({ icon, label, count, active, onClick, indent = 0 }: {
  icon: ReactNode;
  label: string;
  count?: number;
  active: boolean;
  onClick: () => void;
  indent?: number;
}) {
  return (
    <button type="button" className={`vi vv-vi${active ? " on" : ""}`} aria-current={active ? "true" : undefined} onClick={onClick} style={{ paddingLeft: 8 + indent * 12 }}>
      {icon}
      <span className="vv-ellipsis" style={{ minWidth: 0 }}>{label}</span>
      {count !== undefined && <span className="ct">{count}</span>}
    </button>
  );
}

export function LeftPanel({ overview, view, onView, onNewCollection, onNewFolder, collectionMenu, folderMenu }: {
  overview: VaultOverview | undefined;
  view: VaultView;
  onView: (v: VaultView) => void;
  onNewCollection: () => void;
  onNewFolder: () => void;
  collectionMenu: (c: VaultCollectionDto) => MenuItemSpec[];
  folderMenu: (f: VaultFolderDto) => MenuItemSpec[];
}) {
  const is = (v: VaultView) => sameView(view, v);
  const tree = folderTree(overview?.folders ?? []);
  return (
    <nav className="vpanel vv-panel" aria-label="Idea Vault sections">
      <Entry icon={<LayoutGrid size={15} />} label="All items" count={overview?.total} active={is({ kind: "all" })} onClick={() => onView({ kind: "all" })} />
      <Entry icon={<Pin size={15} />} label="Pinned" count={overview?.pinned} active={is({ kind: "pinned" })} onClick={() => onView({ kind: "pinned" })} />
      <Entry icon={<Clock size={15} />} label="Recently added" active={is({ kind: "recentlyAdded" })} onClick={() => onView({ kind: "recentlyAdded" })} />
      <Entry icon={<Pencil size={15} />} label="Recently modified" active={is({ kind: "recentlyModified" })} onClick={() => onView({ kind: "recentlyModified" })} />

      <div className="vh">Collections</div>
      {(overview?.collections ?? []).map((c) => (
        <ContextMenu key={c.id} items={collectionMenu(c)}>
          <div>
            <Entry
              icon={<Layers size={15} />}
              label={c.name}
              count={c.itemCount}
              active={is({ kind: "collection", collectionId: c.id })}
              onClick={() => onView({ kind: "collection", collectionId: c.id })}
            />
          </div>
        </ContextMenu>
      ))}
      <button type="button" className="vi vv-vi" style={{ color: "var(--accent-d)" }} onClick={onNewCollection}>
        <Plus size={15} /> New Collection
      </button>

      <div className="vh">Folders</div>
      {tree.map(({ folder, depth }) => (
        <ContextMenu key={folder.id} items={folderMenu(folder)}>
          <div>
            <Entry
              icon={<Folder size={15} />}
              label={folder.name}
              count={folder.itemCount}
              indent={depth}
              active={is({ kind: "folder", folderId: folder.id })}
              onClick={() => onView({ kind: "folder", folderId: folder.id })}
            />
          </div>
        </ContextMenu>
      ))}
      <button type="button" className="vi vv-vi" style={{ color: "var(--accent-d)" }} onClick={onNewFolder}>
        <Plus size={15} /> New Folder
      </button>

      {(overview?.tags.length ?? 0) > 0 && (
        <>
          <div className="vh">Tags</div>
          {overview!.tags.map((t) => (
            <Entry key={t.tag} icon={<Hash size={14} />} label={t.tag} count={t.count} active={is({ kind: "tag", tag: t.tag })} onClick={() => onView({ kind: "tag", tag: t.tag })} />
          ))}
        </>
      )}
    </nav>
  );
}
