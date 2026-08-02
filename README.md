# GameAP CS2 Addons plugin

A [GameAP](https://gameap.com) panel plugin that adds a **Plugins** tab to
Counter-Strike 2 servers for managing **Metamod:Source** and
**CounterStrikeSharp** — list plugins with live statuses, enable/disable them
persistently, hot load/unload without a restart, edit per-plugin comments,
group plugins, upload new ones, and edit their configs.

## Credits

This project is entirely based on
**[gameap/plugin-goldsrc-addons](https://github.com/gameap/plugin-goldsrc-addons)**
by the GameAP project — the equivalent panel plugin for GoldSource (CS 1.6)
servers managing Metamod and AMX Mod X. The architecture (Rust/WASM backend on
the gameap-plugin-sdk + embedded Vue 3 frontend), the route design, the UI and
most of the code structure come straight from it. All credit for the concept
and design goes to the GameAP authors. This repo is that plugin **vibecoded for
CS2** (with Claude Code).

It pairs with [cs2-addons-manager](https://github.com/booskibro/cs2-addons-manager),
a server-side CounterStrikeSharp plugin providing the same management from the
game console — both maintain the same `plugins_meta.json`, so comments and
groups set in the panel and in game stay in sync. Neither requires the other.

## What the tab shows

- **Platform cards** — Metamod:Source (installed / not active when
  `addons/metamod` exists but `gameinfo.gi` doesn't load it / not installed,
  version via RCON `meta version`) and CounterStrikeSharp (installed state,
  version via RCON `meta list`, plugin counts).
- **Plugin table** — every CounterStrikeSharp plugin found on disk
  (`addons/counterstrikesharp/plugins/*`), merged with live runtime state from
  RCON `css_plugins list`:

  | Status | Meaning |
  |---|---|
  | Running | folder enabled and loaded in the running server |
  | Enabled / Stopped | folder state, when the console is unreachable |
  | Awaiting load | on disk but not loaded (or disabled but still in memory) |
  | Stopped | deliberately unloaded via `css_plugins stop` |
  | Error | the host reports the plugin in an error state |
  | Files missing | broken folder layout, or tracked in `plugins_meta.json` but deleted |

- Plugins are **grouped** by their `Group` from `plugins_meta.json`, with
  inline-editable per-plugin **comments** — the CS2 stand-in for the
  `plugins.ini` sections and comments of the original.

## What the tab does

| Action | How it works |
|---|---|
| Enable / disable (persistent) | Moves the plugin folder between `plugins/` and `plugins/disabled/` — CS2 has no `plugins.ini`; CounterStrikeSharp only loads `plugins/<Name>/<Name>.dll`, so a parked folder stays off across restarts |
| Load / Unload (hot) | RCON `css_plugins load / stop` — no restart needed |
| Install | Upload a plugin `.dll`; the panel creates `plugins/<Name>/` and registers it. Multi-file plugins: unpack via the file manager, then register with the same Upload flow |
| Delete | Removes the plugin folder (recursive); configs under `configs/plugins/` are kept |
| Comments / groups | Stored in `configs/plugins/AddonsManager/plugins_meta.json` (shared with cs2-addons-manager) |
| Config editing | Modal editor for `configs/plugins/<Name>/<Name>.json` |

## HTTP routes (backend)

All admin-only, under `/api/plugins/cs2addons`:

```
GET    /servers/{id}/state               assembled Metamod/CSS state
POST   /servers/{id}/plugins/toggle      {name, enabled}   folder move
POST   /servers/{id}/plugins/attributes  {name, comment, group}
POST   /servers/{id}/plugins             {name, force?}    register upload
DELETE /servers/{id}/plugins             {name} or ?name=  delete
```

The tab is shown only on Source-engine servers and requires the
`plugin:cs2addons:manage` ability (granted to admins automatically); the
backend additionally verifies engine `source` version `2`.

## Install

Copy `cs2-addons.wasm` into the panel's plugins directory (or upload via
Administration → Plugins) and restart the panel — same procedure as the
original plugin.

## Build

Requirements: Rust (the version pinned in `rust-toolchain.toml`, with the
`wasm32-wasip1` target), Node.js 22+, and a checkout of
[gameap/gameap](https://github.com/gameap/gameap) as a **sibling directory
named `gameap-api`** (the frontend depends on its `web/plugin-sdk`). The Rust
side pulls `gameap-plugin-sdk` as a cargo git dependency — no sibling checkout
needed for it.

```sh
# one-time: SDK build (sibling directory)
git clone https://github.com/gameap/gameap ../gameap-api
cd ../gameap-api/web/plugin-sdk
npm ci && npm install --no-save ../frontend/packages/gameap-ui && npx vite build
cd -

make build   # frontend (vite) + wasm (cargo, wasm32-wasip1) → cs2-addons.wasm
make test    # cargo test + vitest
```

On Windows, `make wasm` translates to
`cargo build --target wasm32-wasip1 --release` plus copying
`target/wasm32-wasip1/release/cs2_addons.wasm` to `cs2-addons.wasm`.

## Feature mapping from the original

| plugin-goldsrc-addons (CS 1.6) | plugin-cs2-addons (CS2) |
|---|---|
| Metamod / AMX Mod X status cards | Metamod:Source / CounterStrikeSharp status cards |
| liblist.gam wiring check | gameinfo.gi search-path wiring check |
| plugins.ini parse + comment/uncomment | plugin folder scan + move to `plugins/disabled/` |
| plugins.ini inline comments & sections | `plugins_meta.json` comments & groups |
| "Missing" (in ini, file gone) | "Missing" (in manifest, folder gone) |
| `amxx pause/unpause` via RCON | `css_plugins stop/load` via RCON |
| `.amxx`/`.so`/`.dll` upload + ini registration | `.dll` upload into `plugins/<Name>/` + manifest registration |
| AMXX debug flag | not applicable (no CSS equivalent) |
| `.sma` upload + amxxpc compile | not applicable (C# plugins are compiled with the .NET SDK) |

## License

MIT, same as the original.
