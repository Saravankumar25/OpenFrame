// Links to the Privacy Policy, Terms and refund pages on the public website. Nothing is fetched by
// the app: a page opens in the person's default browser only when they click.

import { ExternalLink } from "lucide-react";
import { Button } from "../../design-system";
import { toast } from "../../app/toast";
import { openUrl } from "../../ipc/client";
import { failureText } from "../licensing/licence";
import { LEGAL_PAGES, legalUrl, type LegalPageId } from "./links";

export function LegalLinks({ only }: { only?: readonly LegalPageId[] }) {
  const pages = only ? LEGAL_PAGES.filter((p) => only.includes(p.id)) : LEGAL_PAGES;
  return (
    <nav aria-label="Legal">
      <ul className="row" style={{ listStyle: "none", margin: 0, padding: 0, gap: 4, flexWrap: "wrap" }}>
        {pages.map(({ id, label }) => (
          <li key={id}>
            <Button
              variant="ghost"
              size="sm"
              icon={<ExternalLink size={13} aria-hidden />}
              onClick={() => {
                void openUrl(legalUrl(id)).catch((e: unknown) => toast.error(failureText(e, "Straft couldn't open that link.")));
              }}
            >
              {label} <span className="sr-only">(opens in your browser)</span>
            </Button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
