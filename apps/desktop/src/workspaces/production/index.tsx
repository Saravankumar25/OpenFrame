// Production workspace (UX §3.21; mock 102 sub-navigation):
//   Overview | Catalog | Locations | Cast & Crew | Moodboards | Storyboards |
//   Shot Lists | Schedule | Daily View | Sides | Reports | Budget | Notes
//
// Tabs are discovered by file convention so modules can add their own tab
// without editing this file: each `./tabs/<id>.tsx` default-exports the tab
// component and exports `tab = { id, label, order }`. Only tabs that exist
// are shown. Reserved order values (keep the mock order):
//   overview 0 · catalog 10 · locations 20 · cast-crew 30 · moodboards 40 ·
//   storyboards 50 · shots 60 · schedule 70 · daily 80 · sides 90 ·
//   reports 100 · budget 110 · notes 120
// The active tab id is `route.sub`; focus targets arrive in `route.params`.
// This index provides the scrolling `.content` layout; tabs must not add their own.

import type { ComponentType } from "react";
import { useNav } from "../../app/stores";
import { ErrorBoundary } from "../../app/shell/ErrorBoundary";

export interface ProductionTabDef {
  id: string;
  label: string;
  order: number;
}

interface TabModule {
  default: ComponentType;
  tab: ProductionTabDef;
}

const modules = import.meta.glob<TabModule>("./tabs/*.tsx", { eager: true });

export const PRODUCTION_TABS: TabModule[] = Object.values(modules)
  .filter((m) => m && typeof m.default === "function" && m.tab && typeof m.tab.id === "string")
  .sort((a, b) => a.tab.order - b.tab.order || a.tab.label.localeCompare(b.tab.label));

export default function ProductionWorkspace() {
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const active = PRODUCTION_TABS.find((t) => t.tab.id === route.sub) ?? PRODUCTION_TABS[0];
  if (!active) return null;
  const Active = active.default;
  return (
    <>
      <nav className="subnav" aria-label="Production" style={{ overflowX: "auto" }}>
        {PRODUCTION_TABS.map((t) => (
          <button
            key={t.tab.id}
            type="button"
            className={t === active ? "active" : undefined}
            aria-current={t === active ? "page" : undefined}
            onClick={() => go({ workspace: "production", sub: t.tab.id })}
          >
            {t.tab.label}
          </button>
        ))}
      </nav>
      {/* Layout is provided here; tabs render their content without a .content wrapper. */}
      <div className="content" style={{ overflow: "auto" }}>
        <ErrorBoundary key={active.tab.id}>
          <Active />
        </ErrorBoundary>
      </div>
    </>
  );
}
