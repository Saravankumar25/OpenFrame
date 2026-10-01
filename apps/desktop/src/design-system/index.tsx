// OpenFrame design system: thin, accessible React components over the UX mock
// CSS vocabulary (openframe.css). Radix provides focus management for
// dialogs/menus; visuals come from the tokens.

import * as RDialog from "@radix-ui/react-dialog";
import * as RMenu from "@radix-ui/react-dropdown-menu";
import * as RContext from "@radix-ui/react-context-menu";
import { X } from "lucide-react";
import {
  forwardRef,
  useId,
  useState,
  type ButtonHTMLAttributes,
  type InputHTMLAttributes,
  type ReactNode,
  type TextareaHTMLAttributes,
} from "react";

const cx = (...c: (string | false | null | undefined)[]) => c.filter(Boolean).join(" ");

// ------------------------------------------------------------------ buttons

type BtnVariant = "default" | "primary" | "danger" | "ghost" | "on";
export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: BtnVariant;
  size?: "md" | "sm" | "xs";
  icon?: ReactNode;
}
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "default", size = "md", icon, className, children, type = "button", ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      className={cx(
        "btn",
        variant === "primary" && "pri",
        variant === "danger" && "dan",
        variant === "ghost" && "ghost",
        variant === "on" && "on",
        size !== "md" && size,
        className,
      )}
      {...rest}
    >
      {icon}
      {children}
    </button>
  );
});

export function IconButton({ label, children, active, ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & { label: string; active?: boolean }) {
  return (
    <button type="button" aria-label={label} title={label} className={cx("iconbtn", active && "on")} {...rest}>
      {children}
    </button>
  );
}

// ------------------------------------------------------------------- forms

export function Field({ label, required, hint, error, children, htmlFor }: {
  label: string;
  required?: boolean;
  hint?: ReactNode;
  error?: string | null;
  children: ReactNode;
  htmlFor?: string;
}) {
  return (
    <div className="field">
      <label htmlFor={htmlFor}>
        {label} {required && <span className="req" aria-hidden>*</span>}
      </label>
      {children}
      {error ? <div className="errtxt" role="alert">{error}</div> : hint ? <div className="hint">{hint}</div> : null}
    </div>
  );
}

export const TextInput = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement> & { invalid?: boolean }>(
  function TextInput({ invalid, className, ...rest }, ref) {
    return <input ref={ref} className={cx("input", className)} aria-invalid={invalid || undefined} {...rest} />;
  },
);

export const TextArea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement> & { invalid?: boolean }>(
  function TextArea({ invalid, className, ...rest }, ref) {
    return <textarea ref={ref} className={cx("textarea", className)} aria-invalid={invalid || undefined} {...rest} />;
  },
);

export function Select<T extends string>({ value, onChange, options, id, ariaLabel }: {
  value: T;
  onChange: (v: T) => void;
  options: readonly { value: T; label: string }[];
  id?: string;
  ariaLabel?: string;
}) {
  return (
    <select id={id} aria-label={ariaLabel} className="select" value={value} onChange={(e) => onChange(e.target.value as T)}>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

/** Segmented control (mock `.seg`). */
export function Segmented<T extends string>({ value, onChange, options, ariaLabel }: {
  value: T;
  onChange: (v: T) => void;
  options: readonly { value: T; label: ReactNode }[];
  ariaLabel: string;
}) {
  return (
    <div className="seg" role="radiogroup" aria-label={ariaLabel}>
      {options.map((o) => (
        <button key={o.value} type="button" role="radio" aria-checked={o.value === value} className={cx(o.value === value && "on")} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Checkbox({ checked, onChange, label, disabled }: { checked: boolean; onChange: (v: boolean) => void; label: ReactNode; disabled?: boolean }) {
  const id = useId();
  return (
    <label htmlFor={id} className="check" style={{ opacity: disabled ? 0.5 : 1 }}>
      <input id={id} type="checkbox" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} style={{ accentColor: "var(--accent)", width: 16, height: 16 }} />
      <span>{label}</span>
    </label>
  );
}

// ------------------------------------------------------------ indicators

export type ChipTone = "default" | "g" | "b" | "r" | "y" | "p" | "a" | "t" | "dark" | "out";
export function Chip({ tone = "default", children, title }: { tone?: ChipTone; children: ReactNode; title?: string }) {
  return <span className={cx("chip", tone !== "default" && tone)} title={title}>{children}</span>;
}

export type BannerTone = "info" | "warn" | "err" | "ok" | "priv";
export function Banner({ tone, children, actions }: { tone: BannerTone; children: ReactNode; actions?: ReactNode }) {
  return (
    <div className={cx("banner", tone)} role={tone === "err" ? "alert" : "status"}>
      <div>{children}</div>
      <span className="sp" />
      {actions}
    </div>
  );
}

export function Dot({ tone = "green" }: { tone?: "green" | "amber" | "red" | "blue" | "gray" | "purple" }) {
  return <span className={cx("dot", tone !== "green" && tone)} aria-hidden />;
}

export function EmptyState({ icon, title, children, actions }: { icon?: ReactNode; title: string; children?: ReactNode; actions?: ReactNode }) {
  return (
    <div className="empty" style={{ padding: 32 }}>
      {icon && <div style={{ color: "var(--muted)" }}>{icon}</div>}
      <div style={{ fontSize: 18, fontWeight: 700, color: "var(--ink)" }}>{title}</div>
      {children && <div style={{ maxWidth: 460, lineHeight: 1.5 }}>{children}</div>}
      {actions && <div className="row" style={{ justifyContent: "center", marginTop: 6 }}>{actions}</div>}
    </div>
  );
}

export function Skeleton({ h = 16, w = "100%" }: { h?: number; w?: number | string }) {
  return <div className="skeleton" style={{ height: h, width: w }} aria-hidden />;
}

export function PageHeader({ title, sub, actions, badge }: { title: ReactNode; sub?: ReactNode; actions?: ReactNode; badge?: ReactNode }) {
  return (
    <div className="ph">
      <div>
        <div className="row">
          <h1>{title}</h1>
          {badge}
        </div>
        {sub && <div className="sub">{sub}</div>}
      </div>
      {actions && <div className="pha">{actions}</div>}
    </div>
  );
}

// ------------------------------------------------------------------ dialogs

export type DialogSize = "sm" | "md" | "lg" | "xl" | "default";
export function Dialog({ open, onOpenChange, title, sub, size = "default", children, footer, footerLeft, dismissible = true }: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  title: ReactNode;
  sub?: ReactNode;
  size?: DialogSize;
  children?: ReactNode;
  footer?: ReactNode;
  footerLeft?: ReactNode;
  /** Blocking decisions (e.g. recovery) must be answered explicitly. */
  dismissible?: boolean;
}) {
  return (
    <RDialog.Root open={open} onOpenChange={(v) => (dismissible || v) && onOpenChange(v)}>
      <RDialog.Portal>
        <RDialog.Overlay className="scrim" />
        <RDialog.Content
          className={cx("dialog", size !== "default" && size)}
          style={{ position: "fixed", left: "50%", top: "50%", transform: "translate(-50%, -50%)", zIndex: 51 }}
          onEscapeKeyDown={(e) => !dismissible && e.preventDefault()}
          onPointerDownOutside={(e) => !dismissible && e.preventDefault()}
          aria-describedby={undefined}
        >
          <div className="dh">
            <div>
              <RDialog.Title asChild>
                <h2>{title}</h2>
              </RDialog.Title>
              {sub && <div className="sub">{sub}</div>}
            </div>
            {dismissible && (
              <RDialog.Close className="iconbtn" aria-label="Close">
                <X size={16} />
              </RDialog.Close>
            )}
          </div>
          <div className="db">{children}</div>
          {(footer || footerLeft) && (
            <div className="df">
              {footerLeft && <div className="l">{footerLeft}</div>}
              {footer}
            </div>
          )}
        </RDialog.Content>
      </RDialog.Portal>
    </RDialog.Root>
  );
}

/** Explicit confirmation with unambiguous verb buttons (UX: never "OK/Cancel" for destructive choices). */
export function ConfirmDialog({ open, onOpenChange, title, children, confirmLabel, cancelLabel = "Cancel", danger, busy, onConfirm, requireText }: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  title: ReactNode;
  children?: ReactNode;
  confirmLabel: string;
  cancelLabel?: string;
  danger?: boolean;
  busy?: boolean;
  onConfirm: () => void;
  /** If set, the user must type this text to enable the action (strong confirmation). */
  requireText?: string;
}) {
  const [typed, setTyped] = useState("");
  const blocked = requireText !== undefined && typed.trim() !== requireText.trim();
  return (
    <Dialog
      open={open}
      onOpenChange={(v) => {
        setTyped("");
        onOpenChange(v);
      }}
      title={title}
      size="md"
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>{cancelLabel}</Button>
          <Button variant={danger ? "danger" : "primary"} disabled={busy || blocked} onClick={onConfirm}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      {children}
      {requireText !== undefined && (
        <Field label="Type the name to confirm">
          <TextInput value={typed} onChange={(e) => setTyped(e.target.value)} placeholder={requireText} autoFocus />
        </Field>
      )}
    </Dialog>
  );
}

// ------------------------------------------------------------------ drawers

export function Drawer({ open, onClose, title, typeLabel, width = "default", children, footer }: {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  typeLabel?: string;
  width?: "default" | "w" | "n";
  children: ReactNode;
  footer?: ReactNode;
}) {
  if (!open) return null;
  return (
    <aside className={cx("drawer", width !== "default" && width)} aria-label={typeof title === "string" ? title : typeLabel}>
      <div className="dh">
        <div className="grow">
          {typeLabel && <div className="ty">{typeLabel}</div>}
          <h2>{title}</h2>
        </div>
        <button type="button" className="iconbtn" aria-label="Close" onClick={onClose}>
          <X size={16} />
        </button>
      </div>
      <div className="db">{children}</div>
      {footer && <div className="df">{footer}</div>}
    </aside>
  );
}

// -------------------------------------------------------------------- menus

export interface MenuItemSpec {
  label: string;
  onSelect?: () => void;
  shortcut?: string;
  danger?: boolean;
  disabled?: boolean;
  icon?: ReactNode;
  separatorBefore?: boolean;
  header?: boolean;
}

/**
 * Menu items, or a function that builds them. A function is only called when
 * the menu opens, so long lists (hundreds of cards, strips or scenes, each with
 * a context menu) don't build every menu on every render.
 */
export type MenuItems = MenuItemSpec[] | (() => MenuItemSpec[]);

/** Rendered inside the (only-when-open) menu content, so lazy items are built on open. */
function MenuItemList<T extends typeof RMenu | typeof RContext>({ ns, items }: { ns: T; items: MenuItems }) {
  return <>{renderItems(ns, typeof items === "function" ? items() : items)}</>;
}

function renderItems<T extends typeof RMenu | typeof RContext>(ns: T, items: MenuItemSpec[]) {
  const Item = ns.Item as typeof RMenu.Item;
  const Sep = ns.Separator as typeof RMenu.Separator;
  const Label = ns.Label as typeof RMenu.Label;
  return items.map((it, i) => (
    <span key={i}>
      {it.separatorBefore && <Sep className="ms" />}
      {it.header ? (
        <Label className="mh">{it.label}</Label>
      ) : (
        <Item className={cx(it.danger && "dan")} disabled={it.disabled} onSelect={() => it.onSelect?.()}>
          {it.icon}
          <span>{it.label}</span>
          {it.shortcut && <span className="k">{it.shortcut}</span>}
        </Item>
      )}
    </span>
  ));
}

/** Dropdown menu attached to a trigger element. */
export function Menu({ trigger, items, align = "start" }: { trigger: ReactNode; items: MenuItems; align?: "start" | "end" }) {
  return (
    <RMenu.Root>
      <RMenu.Trigger asChild>{trigger}</RMenu.Trigger>
      <RMenu.Portal>
        <RMenu.Content className="menu-pop" align={align} sideOffset={4}>
          <MenuItemList ns={RMenu} items={items} />
        </RMenu.Content>
      </RMenu.Portal>
    </RMenu.Root>
  );
}

/** Right-click context menu (FSD §3.6: only actions meaningful for the object). */
export function ContextMenu({ children, items }: { children: ReactNode; items: MenuItems }) {
  return (
    <RContext.Root>
      <RContext.Trigger asChild>{children}</RContext.Trigger>
      <RContext.Portal>
        <RContext.Content className="menu-pop">
          <MenuItemList ns={RContext} items={items} />
        </RContext.Content>
      </RContext.Portal>
    </RContext.Root>
  );
}

export { cx };
