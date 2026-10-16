// Snapshot Fluent 2 theme tokens as stable JSON.
//
// brandWeb is not a public export of @fluentui/tokens@1.0.0-alpha.24. The
// 16-step ramp is copied from packages/tokens/src/global/brandColors.ts at
// microsoft/fluentui@93e324f72f0e (the pin in upstream.toml). The published
// lib/global/brandColors.js matches those values.
//
// The script installs nothing into this repo. npm's one-off runner does not
// put the package on the script's module path, so run it from a throwaway
// directory that has the pinned package:
//
//   mkdir -p /tmp/fluent-tokens && cd /tmp/fluent-tokens
//   npm init -y >/dev/null
//   npm install --no-save --silent @fluentui/tokens@1.0.0-alpha.24
//   node /path/to/fluent2-egui/scripts/fetch-fluent-tokens.mjs
//
// Output (pretty-printed, keys sorted, trailing newline):
//   crates/fluent2-tokens/data/web-light.json
//   crates/fluent2-tokens/data/web-dark.json
//   crates/fluent2-tokens/data/high-contrast.json
//   crates/fluent2-tokens/data/brand-web.json
//   crates/fluent2-tokens/data/brand-marker-light.json
//   crates/fluent2-tokens/data/brand-marker-dark.json

import { createRequire } from "node:module";
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

// Resolve against the working directory, which is the throwaway install, not
// this repo. See the header for the command.
const require = createRequire(pathToFileURL(join(process.cwd(), "package.json")));
// require.resolve returns the CJS build. Its package "exports" hides the ESM
// build, and importing the CJS file only exposes `default`. Step from the
// resolved file to lib/index.js and import that URL directly.
const cjs = require.resolve("@fluentui/tokens");
const esm = join(dirname(cjs), "..", "lib", "index.js");
const tokensUrl = pathToFileURL(esm);
const {
  createDarkTheme,
  createLightTheme,
  teamsHighContrastTheme,
  webDarkTheme,
  webLightTheme,
} = await import(tokensUrl);

const outDir = join(dirname(fileURLToPath(import.meta.url)), "..", "crates", "fluent2-tokens", "data");

/** 16-step brandWeb ramp from brandColors.ts at the pinned fluentui rev. */
const brandWeb = {
  10: "#061724",
  20: "#082338",
  30: "#0a2e4a",
  40: "#0c3b5e",
  50: "#0e4775",
  60: "#0f548c",
  70: "#115ea3",
  80: "#0f6cbd",
  90: "#2886de",
  100: "#479ef5",
  110: "#62abf5",
  120: "#77b7f7",
  130: "#96c6fa",
  140: "#b4d6fa",
  150: "#cfe4fa",
  160: "#ebf3fc",
};

const steps = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160];

/** Marker ramp: step N is #0000NN, NN = N/10 as two hex digits. */
const marker = {};
for (const step of steps) {
  marker[step] = "#0000" + (step / 10).toString(16).padStart(2, "0");
}

function flatten(theme) {
  const flat = {};
  for (const [key, value] of Object.entries(theme)) {
    if (value !== null && typeof value === "object") {
      throw new Error(`${key} is not a scalar (${typeof value})`);
    }
    flat[key] = value;
  }
  return flat;
}

function writeSorted(name, object) {
  const keys = Object.keys(object).sort();
  const ordered = {};
  for (const key of keys) {
    ordered[key] = object[key];
  }
  const path = join(outDir, name);
  writeFileSync(path, JSON.stringify(ordered, null, 2) + "\n");
}

writeSorted("web-light.json", flatten(webLightTheme));
writeSorted("web-dark.json", flatten(webDarkTheme));
writeSorted("high-contrast.json", flatten(teamsHighContrastTheme));
writeSorted(
  "brand-web.json",
  Object.fromEntries(steps.map((step) => [String(step), brandWeb[step]])),
);
writeSorted("brand-marker-light.json", flatten(createLightTheme(marker)));
writeSorted("brand-marker-dark.json", flatten(createDarkTheme(marker)));
