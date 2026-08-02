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

## Quick start

1. Get `cs2-addons.wasm` — download the `cs2-addons-wasm` artifact from the
   latest [Actions run](../../actions), or build it yourself (see
   [Build](#build)).
2. In your GameAP panel: **Administration → Plugins → upload** the `.wasm`
   (or copy it into the panel's plugins directory by hand).
3. Restart the panel.
4. Open any **Counter-Strike 2** server — a **Plugins** tab appears next to
   the server's other tabs (admins only).

## Requirements

**Panel:** a GameAP installation recent enough to support WASM panel plugins
(the current [gameap/gameap](https://github.com/gameap/gameap) line — same
requirement as the original goldsrc-addons plugin).

**Game server** (for full functionality; the tab degrades gracefully without):

- A CS2 server created in GameAP with the stock `cs2` game (engine
  `source` v2 — that's what the tab is gated on).
- [Metamod:Source 2.x](https://www.sourcemm.net/downloads.php?branch=master)
  in `game/csgo/addons/metamod`, with the search path added to `gameinfo.gi`.
  The Metamod card shows **Not active** if the folder exists but the
  `gameinfo.gi` line is missing.
- [CounterStrikeSharp](https://github.com/roflmuffin/CounterStrikeSharp)
  (the *with runtime* build) in `game/csgo/addons/counterstrikesharp`.
- An **RCON password** configured on the server in GameAP. Without it the tab
  still works — you just lose live data (versions, Running/Stopped states,
  hot load/unload); everything file-based keeps working and a hint line
  explains what's unavailable.

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

## Using the tab

**The two switches mean different things:**

- The **On/Off switch** on a row is *persistent*: it moves the plugin folder
  in or out of `plugins/disabled/`. It does not touch the running server —
  after switching, either use Load/Unload for an immediate effect or let the
  next restart apply it. (This is the exact analogue of commenting a
  `plugins.ini` line in the original.)
- The **Load / Unload button** is *immediate but temporary*: it runs
  `css_plugins load/stop` over RCON. An unloaded plugin comes back at the next
  restart unless you also switch it off.

Typical flows:

- **Install a plugin (single DLL):** *Upload file* → pick the `.dll` → the
  panel creates `plugins/<Name>/`, uploads it, and registers it. Press *Load*
  on the new row to start it without a restart.
- **Install a multi-file plugin (zip release):** unpack the zip so that
  `plugins/<Name>/<Name>.dll` exists (the panel file manager can upload &
  extract), then *Upload file* with just the main `.dll` to register it — the
  upload overwrites the same file harmlessly and creates the manifest entry.
- **Update a plugin:** *Upload file* with the new `.dll` (the orange
  *Overwrite* state confirms it's an update), then *Load*/restart.
- **Retire a misbehaving plugin:** switch it **Off** (persists), or *Delete*
  to remove its folder entirely — configs under `configs/plugins/` survive
  deletion.
- **Annotate:** click the pencil on a row to edit its comment; comments and
  groups live in `plugins_meta.json` and appear both here and in the
  `css_addons` console commands if you run
  [cs2-addons-manager](https://github.com/booskibro/cs2-addons-manager).
- **Edit a config:** rows with a gear button have
  `configs/plugins/<Name>/<Name>.json`; the modal edits it in place. Other
  config files: use the file manager link under the table.
- **Bulk:** select rows with the checkboxes for enable/disable/delete in one go.

## HTTP routes (backend)

All admin-only, under `/api/plugins/mnzteylemrxw4`:

```
GET    /servers/{id}/state               assembled Metamod/CSS state
POST   /servers/{id}/plugins/toggle      {name, enabled}   folder move
POST   /servers/{id}/plugins/attributes  {name, comment, group}
POST   /servers/{id}/plugins             {name, force?}    register upload
DELETE /servers/{id}/plugins             {name} or ?name=  delete
```

The tab is shown only on Source-engine servers and requires the
`plugin:mnzteylemrxw4:manage` ability (granted to admins automatically); the
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

## Troubleshooting

- **The Plugins tab doesn't appear** — check, in order: the plugin is listed
  under Administration → Plugins and the panel was restarted; the server's
  game is `cs2` (the tab is gated on engine `source`, and the backend
  additionally requires engine version `2` — a custom game entry with a
  different engine string won't show it); your user has admin rights (the
  `plugin:mnzteylemrxw4:manage` ability is granted to admins automatically).
- **"could not locate the game directory"** (422 on load) — the server dir
  doesn't contain `game/csgo/gameinfo.gi` (or any `game/*/gameinfo.gi`).
  The server probably isn't installed yet or uses a nonstandard layout.
- **Metamod shows "Not active"** — `addons/metamod` exists but `gameinfo.gi`
  has no `csgo/addons/metamod` search path. Add it via the file manager
  (CS2 updates are known to revert this file).
- **A grey hint line about RCON** — live statuses are unavailable for the
  stated reason (offline / no password / wrong password / empty response).
  Fix the RCON password in the server settings; file operations work
  regardless.
- **Toggle fails with "already exists; refusing to overwrite"** — both
  `plugins/<Name>/` and `plugins/disabled/<Name>/` exist (usually after a
  manual copy). Delete or rename one of them in the file manager.
- **A row shows "Files missing"** — either the folder lacks its
  `<Name>/<Name>.dll` (broken layout — fix the folder name or dll name), or
  the plugin is tracked in `plugins_meta.json` but its folder was deleted
  (delete the row to forget it).
- **Where is the manifest?**
  `game/csgo/addons/counterstrikesharp/configs/plugins/AddonsManager/plugins_meta.json`
  — plain JSON, safe to hand-edit; the path is shared with the
  cs2-addons-manager server plugin on purpose.

## Uninstall

Remove the plugin in Administration → Plugins (or delete the `.wasm` from the
panel's plugins directory) and restart the panel. Nothing on the game servers
is changed by uninstalling; anything you disabled stays in
`plugins/disabled/` until you move it back.

## Repository layout

```
src/                        Rust backend (compiled to wasm32-wasip1)
├── lib.rs                  plugin entry: info, routes, frontend bundle
├── router.rs               route table + dispatch
├── handlers/               one file per route (+ tests.rs against a mock node)
├── source2/                CS2 domain: paths, gamedir, gameinfo.gi, plugins_meta.json
├── host_api.rs             trait over the SDK host calls + MockHost for native tests
└── http.rs                 JSON response/error helpers
frontend/                   Vue 3 frontend (vite lib build, embedded into the wasm)
├── src/index.ts            plugin definition, translations (en/ru), tab registration
├── src/components/         ModsTab, PlatformCard, PluginList, InstallModal, ConfigModal
├── src/lib/                rcon-parse, status logic, naming (unit-tested)
└── src/api/                clients for the wasm routes and existing panel endpoints
```

Tests: `cargo test` (32 backend tests run natively against the mock node — no
panel needed) and `cd frontend && npm test` (22 vitest tests for the parsers
and status logic).

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
