// Settings opened from Home (owner change list item 6): only the settings that belong to this
// computer, not to a project. Inside a project the same gear opens the Settings page, which shows
// the project tabs and these global ones.

import { useState } from "react";
import { Button, Dialog } from "../../design-system";
import { AppearancePanel } from "../../features/appearance/AppearancePanel";
import { ShortcutsSettings } from "../../workspaces/settings/Shortcuts";
import { AboutSection } from "../../workspaces/settings/About";
import { LicenceSection } from "../../workspaces/settings/Licence";
import { SupportSection } from "../../workspaces/settings/SupportDiagnostics";

export type GlobalSettingsTab = "appearance" | "shortcuts" | "about";

const TABS: { id: GlobalSettingsTab; label: string }[] = [
  { id: "appearance", label: "Appearance" },
  { id: "shortcuts", label: "Keyboard shortcuts" },
  { id: "about", label: "About and support" },
];

export function GlobalSettingsDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [tab, setTab] = useState<GlobalSettingsTab>("appearance");
  const [recording, setRecording] = useState(false);
  return (
    <Dialog
      open={open}
      onOpenChange={(v) => !v && onClose()}
      title="Settings"
      sub="These settings apply to OpenFrame on this computer. Open a project to change its own settings."
      size="xl"
      dismissible={!recording}
      footer={<Button variant="primary" onClick={onClose} disabled={recording}>Done</Button>}
    >
      <div className="subnav" role="tablist" aria-label="Settings" style={{ margin: "0 -18px 12px", padding: "0 18px" }}>
        {TABS.map((t) => (
          <button key={t.id} type="button" role="tab" aria-selected={tab === t.id} className={tab === t.id ? "active" : undefined} onClick={() => setTab(t.id)}>
            {t.label}
          </button>
        ))}
      </div>
      {tab === "appearance" ? (
        <AppearancePanel />
      ) : tab === "shortcuts" ? (
        <ShortcutsSettings embedded onRecordingChange={setRecording} />
      ) : (
        <div className="col">
          <LicenceSection />
          <AboutSection />
          <SupportSection />
        </div>
      )}
    </Dialog>
  );
}
