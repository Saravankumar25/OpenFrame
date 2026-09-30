// OpenFrame Offline AI manifest signing tool (release engineering only).
//
//   node sign-manifest.mjs --generate-key <private-key-out.pem>
//       Creates an Ed25519 key pair. Prints the base64 public key to embed
//       (crates/openframe-ai/keys/manifest-dev.pub for development builds, or
//       the OPENFRAME_MANIFEST_PUBLIC_KEY build variable for releases).
//       NEVER commit the private key.
//
//   node sign-manifest.mjs <manifest.json> <private-key.pem>
//       Signs the exact bytes of manifest.json and writes manifest.json.sig
//       (base64 Ed25519 signature) next to it.
import { generateKeyPairSync, createPrivateKey, createPublicKey, sign } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";

function rawPublicKeyB64(publicKey) {
  const der = publicKey.export({ type: "spki", format: "der" });
  return der.subarray(der.length - 32).toString("base64");
}

const args = process.argv.slice(2);
if (args[0] === "--generate-key" && args[1]) {
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  writeFileSync(args[1], privateKey.export({ type: "pkcs8", format: "pem" }), { mode: 0o600 });
  console.log(rawPublicKeyB64(publicKey));
} else if (args.length === 2) {
  const [manifestPath, keyPath] = args;
  const bytes = readFileSync(manifestPath);
  JSON.parse(bytes.toString("utf8")); // refuse to sign malformed JSON
  const key = createPrivateKey(readFileSync(keyPath));
  const signature = sign(null, bytes, key).toString("base64");
  writeFileSync(`${manifestPath}.sig`, signature);
  console.log(`signed ${manifestPath} with key ${rawPublicKeyB64(createPublicKey(key))}`);
} else {
  console.error("usage: sign-manifest.mjs --generate-key <out.pem> | <manifest.json> <private-key.pem>");
  process.exit(2);
}
