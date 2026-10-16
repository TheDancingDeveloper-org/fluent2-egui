# Fluent 2 token snapshot

Committed JSON so later crates can generate Rust without Node. Values come from
npm `@fluentui/tokens@1.0.0-alpha.24`, which matches `microsoft/fluentui`
`packages/tokens` at `93e324f72f0e` (the `fluentui` pin in `upstream.toml`).
Licence: MIT, Copyright (c) Microsoft Corporation. See `ATTRIBUTION.md`.

| File | Contents |
|---|---|
| `web-light.json` | `webLightTheme`, flattened. |
| `web-dark.json` | `webDarkTheme`, flattened. |
| `high-contrast.json` | `teamsHighContrastTheme`, flattened. |
| `brand-web.json` | The 16-step `brandWeb` ramp, keys `"10"`..`"160"`. Not a public export of the npm package; copied from `packages/tokens/src/global/brandColors.ts` at the pinned rev. |
| `brand-marker-light.json` | `createLightTheme(marker)`. |
| `brand-marker-dark.json` | `createDarkTheme(marker)`. |

`marker` is a brand ramp whose step N is `#0000NN`, with NN = N/10 as two hex digits (`10` → `#000001`, `160` → `#000010`). A later item uses the marker colours to see which alias tokens come from which brand step.

Regenerate (the script installs nothing into this repo):

```
mkdir -p /tmp/fluent-tokens && cd /tmp/fluent-tokens
npm init -y
npm install --no-save @fluentui/tokens@1.0.0-alpha.24
node /path/to/fluent2-egui/scripts/fetch-fluent-tokens.mjs
```

`npx -p` does not put the package on the local script's module path, so the install lives in a throwaway directory and nothing is written into this repo.

JSON is pretty-printed with sorted keys and a trailing newline, so two runs are byte-identical.
