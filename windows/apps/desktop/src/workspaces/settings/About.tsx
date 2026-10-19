// Settings > About: the app's name and version, and "Internal build, not for sale." on the
// internal team builds (ADR-0023, ADR-0024). Store builds show the version only.

import { useOp } from "../../ipc/query";
import type { AppInfo } from "../../ipc/generated/AppInfo";
import type { EntitlementStatus } from "../../ipc/generated/EntitlementStatus";

/** The words for the internal team builds, the same as the Licence card. */
export const INTERNAL_BUILD_TEXT = "Internal build, not for sale.";

export function AboutSection() {
  const info = useOp<AppInfo>("app.info", {}, []);
  const licence = useOp<EntitlementStatus>("entitlement.status", {}, []);
  const internal = licence.data?.channel === "directInstaller";
  return (
    <section className="card pad" style={{ maxWidth: 720, marginTop: 16 }} aria-labelledby="about-heading">
      <h2 id="about-heading" className="h4">About OpenFrame Studio</h2>
      <p className="sm" style={{ margin: "0 0 4px" }}>
        Version {info.data ? info.data.appVersion : "…"}
      </p>
      {internal && (
        <p className="sm" style={{ margin: 0 }}>
          <b>{INTERNAL_BUILD_TEXT}</b>
        </p>
      )}
    </section>
  );
}
