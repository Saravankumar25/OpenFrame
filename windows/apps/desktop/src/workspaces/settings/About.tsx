// Settings > About: the app's name and version, the publisher, how to contact us and the third-party
// licences. The internal team builds add "Internal build, not for sale." (ADR-0023, ADR-0024).

import { useState } from "react";
import { Button, Dialog } from "../../design-system";
import { useOp } from "../../ipc/query";
import type { AppInfo } from "../../ipc/generated/AppInfo";
import type { EntitlementStatus } from "../../ipc/generated/EntitlementStatus";
import { openContact } from "../../features/licensing/licence";
import { LegalLinks } from "../../features/legal/LegalLinks";
import { ThirdPartyNotices } from "./ThirdPartyNotices";
import licenceText from "../../../../../THIRD_PARTY_LICENSES.md?raw";

/** The words for the internal team builds, the same as the Licence card. */
export const INTERNAL_BUILD_TEXT = "Internal build, not for sale.";

const PUBLISHER = "Vighnajith Advisories Private Limited";
const ADDRESS = "12/1, Ground Floor, 22nd Main Road, Bengaluru, Karnataka 560078, India";
// The Gemma Terms of Use and the Prohibited Use Policy bind anyone who uses the model; no web
// address here, a new address needs the owner's list (scripts/app-addresses.json).
const AI_NOTICE = "Straft AI uses Gemma, provided under the Gemma Terms of Use and its Prohibited Use Policy, and llama.cpp.";

export function AboutSection() {
  const info = useOp<AppInfo>("app.info", {}, []);
  const licence = useOp<EntitlementStatus>("entitlement.status", {}, []);
  const [licencesOpen, setLicencesOpen] = useState(false);
  const internal = licence.data?.channel === "directInstaller";
  return (
    <section className="card pad" style={{ maxWidth: 720, marginTop: 16 }} aria-labelledby="about-heading">
      <h2 id="about-heading" className="h4">About Straft Studio</h2>
      <p className="sm" style={{ margin: "0 0 4px" }}>
        Version {info.data ? info.data.appVersion : "…"}
      </p>
      {internal && (
        <p className="sm" style={{ margin: "0 0 4px" }}>
          <b>{INTERNAL_BUILD_TEXT}</b>
        </p>
      )}
      <p className="sm" style={{ margin: "0 0 4px" }}>Publisher: {PUBLISHER}</p>
      <p className="sm" style={{ margin: "0 0 4px" }}>{ADDRESS}</p>
      <p className="sm" style={{ margin: "0 0 4px" }}>Copyright 2026 {PUBLISHER}</p>
      <p className="sm" style={{ margin: "0 0 8px" }}>{AI_NOTICE}</p>
      <div className="row" style={{ gap: 8 }}>
        <Button size="sm" onClick={openContact}>Contact us</Button>
        <Button size="sm" onClick={() => setLicencesOpen(true)}>Third-party licences</Button>
      </div>
      <LegalLinks />
      <Dialog open={licencesOpen} onOpenChange={setLicencesOpen} title="Third-party licences" size="xl">
        <ThirdPartyNotices />
        <h3 className="sm" style={{ margin: "16px 0 4px" }}>Inventory</h3>
        <pre className="sm selectable-text" tabIndex={0} aria-label="Licence text" style={{ whiteSpace: "pre-wrap", maxHeight: "60vh", overflow: "auto", margin: 0 }}>
          {licenceText}
        </pre>
      </Dialog>
    </section>
  );
}
