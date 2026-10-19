// Advanced settings: breakdown category colours and icons (depth spec section 23).
// Normal users never need this page; the defaults are chosen for contrast and clarity.
// The label and tooltip of a category never change, so colour is never the only signal.

import type { CSSProperties } from "react";
import { RotateCcw } from "lucide-react";
import { useCommand } from "../../ipc/query";
import type { BreakdownCategoryStyles } from "../../ipc/generated/BreakdownCategoryStyles";
import type { BreakdownCategoryStyleArgs } from "../../ipc/generated/BreakdownCategoryStyleArgs";
import { Banner, Button, Chip, PageHeader, Skeleton } from "../../design-system";
import { useCanEdit } from "../../api/production";
import { toast } from "../../app/toast";
import { CATEGORY_ICONS, CategoryMark, lookOf, useCategoryStyles } from "../breakdown/categoryStyle";

export function BreakdownCategorySettings() {
  const q = useCategoryStyles();
  const canEdit = useCanEdit();
  const set = useCommand<BreakdownCategoryStyleArgs, BreakdownCategoryStyles>("breakdown.set_category_style", {
    onSuccess: (_, a) => toast.undoable(`Changed the look of ${a.category}`),
  });
  const reset = useCommand<Record<string, never>, BreakdownCategoryStyles>("breakdown.reset_category_styles", {
    onSuccess: () => toast.undoable("Restored the default category colours and icons"),
  });
  const data = q.data;
  return (
    <div className="content">
      <PageHeader
        title="Advanced"
        sub="Breakdown category colours and icons. Every category also keeps its name and description, so colour is never the only signal."
        actions={
          canEdit && (
            <Button icon={<RotateCcw size={14} />} disabled={!data?.categories.some((c) => c.customized) || reset.isPending} onClick={() => reset.mutate({})}>
              Restore Defaults
            </Button>
          )
        }
      />
      {q.isLoading && <Skeleton h={200} />}
      {q.error && <Banner tone="err">{q.error.message}</Banner>}
      {data && (
        <div className="card" style={{ maxWidth: 860 }}>
          {data.categories.map((c) => {
            const look = lookOf(c.category, data.categories);
            return (
              <section key={c.category} className="li" aria-label={`${c.label} look`} style={{ alignItems: "flex-start", gap: 14 }}>
                <div style={{ width: 190, flex: "none" }}>
                  <CategoryMark look={look} />
                  <div className="xs muted" style={{ marginTop: 4 }}>{c.tooltip}</div>
                  {c.customized && <Chip tone="a">Changed</Chip>}
                </div>
                <div className="grow">
                  <div className="cat-swatches" role="group" aria-label={`${c.label} colour`}>
                    {data.palette.map((p) => (
                      <button
                        key={p.token}
                        type="button"
                        aria-pressed={c.color === p.token}
                        aria-label={p.label}
                        title={p.label}
                        disabled={!canEdit || set.isPending}
                        style={{ "--cat": p.hex } as CSSProperties}
                        onClick={() => c.color !== p.token && set.mutate({ category: c.category, color: p.token, icon: null })}
                      />
                    ))}
                  </div>
                  <div className="cat-icons" role="group" aria-label={`${c.label} icon`} style={{ marginTop: 8, "--cat": c.colorHex } as CSSProperties}>
                    {data.icons.map((key) => {
                      const Icon = CATEGORY_ICONS[key];
                      if (!Icon) return null;
                      return (
                        <button
                          key={key}
                          type="button"
                          aria-pressed={c.icon === key}
                          aria-label={`Icon ${key.replace("-", " ")}`}
                          title={key.replace("-", " ")}
                          disabled={!canEdit || set.isPending}
                          onClick={() => c.icon !== key && set.mutate({ category: c.category, color: null, icon: key })}
                        >
                          <Icon size={15} aria-hidden />
                        </button>
                      );
                    })}
                  </div>
                </div>
              </section>
            );
          })}
        </div>
      )}
    </div>
  );
}
