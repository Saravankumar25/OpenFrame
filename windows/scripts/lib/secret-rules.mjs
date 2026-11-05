// Secret patterns shared by scripts/check-secrets.mjs (repository and history) and
// scripts/scan-installer.mjs (built files). ADR-0030 decision 6: a hit blocks the build.
//
// A line that must hold a look-alike on purpose (a test of these rules, a documented example)
// carries the marker `secret-scan: allow` on the same line. Test code builds fake secrets by
// joining strings so the literal never appears in the repository.

export const RULES = [
  { id: "private-key", re: /-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP |ENCRYPTED )?PRIVATE KEY( BLOCK)?-----/ },
  { id: "aws-access-key", re: /\b(?:AKIA|ASIA)[0-9A-Z]{16}\b/ },
  { id: "github-token", re: /\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{60,})\b/ },
  { id: "anthropic-key", re: /\bsk-ant-[A-Za-z0-9_-]{20,}/ },
  { id: "openai-key", re: /\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}T3BlbkFJ[A-Za-z0-9_-]{20,}/ },
  { id: "google-api-key", re: /\bAIza[0-9A-Za-z_-]{35}(?![0-9A-Za-z_-])/ },
  // Tauri updater and minisign private keys, as text and in the base64 form Tauri stores.
  { id: "updater-private-key", re: /(?:rsign|minisign) encrypted secret key|dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5/ }, // secret-scan: allow (the pattern itself)
  { id: "slack-token", re: /\bxox[abposr]-[A-Za-z0-9-]{10,}/ },
  { id: "stripe-live-key", re: /\b(?:sk|rk)_live_[0-9A-Za-z]{20,}/ },
  { id: "hugging-face-token", re: /\bhf_[A-Za-z0-9]{34,}\b/ },
  { id: "npm-token", re: /\bnpm_[A-Za-z0-9]{36}\b/ },
];

/** Files that must never be committed, whatever they contain. */
export const SECRET_FILE = /(^|\/)(\.env(\.[^/]*)?|id_(rsa|dsa|ecdsa|ed25519)|[^/]+\.(pem|pfx|p12|key|keystore|jks|snk))$/i;
/** Example files that document the shape of a secret file without holding one. */
export const SECRET_FILE_EXAMPLE = /(^|\/)\.env\.(example|sample|template)$/i;

export const ALLOW_MARKER = "secret-scan: allow";

/** Rule ids that match one line, unless the line carries the allow marker. */
export function lineHits(line) {
  if (line.includes(ALLOW_MARKER)) return [];
  return RULES.filter((r) => r.re.test(line)).map((r) => r.id);
}

/** True when a repository path names a secret file. */
export const isSecretFile = (rel) => SECRET_FILE.test(rel) && !SECRET_FILE_EXAMPLE.test(rel);
