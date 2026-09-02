# GameAP CS2 Addons plugin

A [GameAP](https://gameap.com) panel plugin that adds a **Plugins** tab to
Counter-Strike 2 servers for managing **Metamod:Source** and
**CounterStrikeSharp** — list plugins with live statuses, enable/disable them
persistently, hot load/unload without a restart, edit per-plugin comments,
group plugins, upload new ones, and edit their configs.

## Features

Beyond the basics above:

- **One-click platform installs.** Metamod:Source and CounterStrikeSharp are
  installed and updated from their latest release, downloaded and unpacked on
  the node, with an update button when a newer one ships.
- **`gameinfo.gi` repair.** A *Fix* button, plus a 6-hour sweep that re-wires
  every CS2 server automatically — game updates keep stripping the Metamod
  search path.
- **A CS2-tolerant RCON protocol** replacing the panel's built-in Source client
  for `cs2` servers: multi-packet replies, oversized packets, both auth
  response shapes and CS2's unsolicited console output are all handled rather
  than erroring out.
- **A plugin catalog** (MatchZy, CS2-SimpleAdmin, Retakes, WeaponPaints,
  K4-System) installable straight from GitHub releases, with update badges on
  installed rows.
- **Zip installs.** Drop any release archive on the Upload dialog; the layout
  is detected — `addons/`-rooted, `plugins/` + `shared/`, `Name/Name.dll` or a
  loose dll — and unpacked to the right place, including the contract
  assemblies under `shared/` that some plugins need to load at all.
- **Snapshots** of `plugins/`, `configs/plugins/` and `shared/` as tarballs on
  the server, taken automatically before anything overwrites files and
  restorable in one click. Also the transfer format for copying a setup between
  servers.
- **Binary Metamod plugins** (`addons/metamod/*.vdf` aliases) with the same
  on/off switches — with CounterStrikeSharp's own alias guarded, since
  switching that one off unloads the entire platform.
- **A CounterStrikeSharp admins editor** for `configs/admins.json` and
  `admin_groups.json`, as an editable table.
- **A log viewer** with filtering, following and a download link, and a **Doctor**
  dialog running every health check in one pass — launch parameters, RCON,
  `gameinfo.gi` wiring, folder placement and layout, shared assemblies, manifest
  orphans, ambiguous `.vdf` aliases, plugins that failed to load, and leftover
  downloads.
- **An audit history** of panel actions, and a restart banner when changes are
  genuinely waiting for one.

Version-by-version history is in [CHANGELOG.md](CHANGELOG.md).

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

1. Get `cs2-addons.wasm` from the [latest release](../../releases/latest) —
   it is attached to every release as a downloadable asset. (Unreleased
   builds are also produced by CI as the `cs2-addons-wasm` artifact of an
   [Actions run](../../actions), but those expire and need a GitHub login to
   download.) You can also build it yourself — see [Build](#build).
2. In your GameAP panel: **Administration → Plugins → upload** the `.wasm`
   (or copy it into the panel's plugins directory by hand).
3. Restart the panel.
4. Open any **Counter-Strike 2** server — a **Plugins** tab appears next to
   the server's other tabs (admins only).

## Requirements

**Panel:** a GameAP installation recent enough to support WASM panel plugins
AND the `gameap-net` / `gameap-scheduler` host modules (current
[gameap/gameap](https://github.com/gameap/gameap) main). Since 0.2.0 the
plugin imports both modules, so it will not load on older panels; no 0.1.x
build is published, so an older panel means building one from the `0.1.0`
history yourself. Related panel settings, all default-on:
`PLUGINS_NET_ENABLED=true` (the CS2 RCON protocol; with `false` the panel
falls back to its built-in Source client), and plugin HTTP with `https`
allowed (release lookups query `api.github.com` and `mms.alliedmods.net`).
Those are the `v4.5.0-rc.1` spellings; a panel before that release calls them
`PLUGIN_NET_ENABLED` and so on. rc.1 still accepts the older names for one
release, with a deprecation warning in the log.

**Node:** platform installs and snapshots run `curl`/`wget`, `tar` and
`unzip` (fallbacks: `python3 -m zipfile`, `busybox unzip`) on a **linux**
node. A stock Debian/Ubuntu box has everything except possibly `unzip` -
`apt install unzip` covers the CounterStrikeSharp bundle.

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

## GameAP 4.5

The plugin runs unchanged on 4.5. Three things there are worth knowing, and one
of them is a limit you can actually hit.

**Permissions.** 4.5 gates the privileged host libraries behind grants a plugin
declares in its manifest, and this one declares three:

| Grant | For |
| --- | --- |
| `node_commands` | `gameap-nodecmd.execute_command` - the node-side unpack and platform installs |
| `files` | the `gameap-nodefs` writes (`chmod`, `mk_dir`, `move`, `remove`, `upload`). It covers `files_read`, so the reads are not listed separately - the panel treats a broader grant as satisfying a narrower one and drops the subsumed entry |
| `manage_servers` | `gameap-servercontrol.restart_server` |

Nothing else needs one. `gameap-nodes.get_node` and `gameap-servers`'
`find_servers`/`get_server` are read-only and ungated - it is the *writes* on
those modules that need `manage_nodes` and `manage_servers`, and this plugin
makes none. `gameap-http`, `gameap-storage`, `gameap-scheduler`, `gameap-games`
and `gameap-log` are not gated at all.

The list is not a judgement call: the panel derives what a plugin *uses* from
its wasm import section, matched against its own policy table, and shows
anything used-but-undeclared as a missing grant. `REQUIRED_PERMISSIONS` in
`src/lib.rs` is checked against the panel's twelve known names by a unit test,
because `ParsePluginPermissions` silently **drops** a name it does not
recognise - a typo there would not fail an install, it would grant less than
intended and surface much later as a denied call.

`PLUGINS_PERMISSIONS_ENFORCE` still defaults to `false` in 4.5, so nothing is
enforced yet; a later release turns it on.

> **Panel variable names changed during 4.5.** Every plugin setting was renamed
> `PLUGIN_*` → `PLUGINS_*` in `v4.5.0-rc.1` (PR #85). The old spelling keeps
> working for **one release** - the panel applies it and logs a deprecation
> warning naming the replacement, and the new name wins if both are set - so
> this is worth getting ahead of rather than racing. This README uses the rc.1
> names throughout. On `v4.5.0-beta.1` drop the `S`:
> `PLUGIN_NODEFS_MAX_INLINE`. On 4.4.x none of these exist at all, because the
> limits they control arrived with 4.5.

**A 32 MiB ceiling on single file transfers.** `PLUGINS_NODEFS_MAX_INLINE`
caps what one `gameap-nodefs` download or upload may carry, and there was no
equivalent before. It binds two paths here:

- **Zip installs.** The plugin's own `MAX_ARCHIVE_BYTES` is 33,554,432 bytes -
  the panel's default `32M` to the byte, since its size parser reads every
  suffix as binary. Both refuse at 33,554,433. The plugin stats the file first,
  so its own message is what you see rather than the panel's generic *file too
  large*. Note the margin is exactly **zero**: lowering
  `PLUGINS_NODEFS_MAX_INLINE` puts the panel's limit under this gate, and the
  refusal then comes from the panel with less to say. Both constants derive from
  one `PANEL_MAX_INLINE_BYTES` so they cannot drift apart.
- **A single file inside an archive.** Extraction bounds the *total*
  uncompressed size at twice the cap and bounds no individual member, so an
  oversized file can arrive inside a perfectly acceptable archive. Since entries
  are uploaded one at a time, the panel would refuse that one partway through
  and leave a half-installed plugin. Every entry is therefore checked **before
  anything is written**, and the whole archive refused with `ENTRY_TOO_LARGE`.

**Logs over 32 MiB are the one real regression, and it is not fixable here.**
The log tail is read by downloading the whole file and keeping the last 256 KB.
That was fine while the daemon had no ranged reads. In 4.5 a whole-file download
past the inline limit is **refused outright** rather than truncated, so a server
whose newest CounterStrikeSharp log has grown past 32 MiB loses the logs view,
and the doctor route with it - both with the panel's *file too large* error.

4.5 shipped the fix in the same release: `offset` and `length` on the nodefs
`DownloadRequest`, which is exactly the windowed read this wants. The plugin
cannot reach it. Those fields exist in the panel's own proto, while the
`gameap-proto` commit this crate pins - still the tip of that repo's `main` -
declares `DownloadRequest` as `node_id` + `path` alone. When the SDK catches up
this becomes a windowed read of the tail and the ceiling goes away; until then,
rotate the logs or raise `PLUGINS_NODEFS_MAX_INLINE`.

**Rate limits** are new and on by default - nodefs 50/s with a burst of 200,
nodecmd and servercontrol 5/s burst 20, http 20/s burst 50. The install and
repair flows do bursts of nodefs calls, and this is the one limit here that has
*not* been measured: unlike the size caps it cannot be read off the source, and
wants a real install against a live panel.

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
GET    /servers/{id}/state               assembled Metamod/CSS state (+ vdf plugins)
POST   /servers/{id}/plugins/toggle      {name, enabled}   folder move
POST   /servers/{id}/plugins/attributes  {name, comment, group}
POST   /servers/{id}/plugins             {name, force?}    register upload
DELETE /servers/{id}/plugins             {name} or ?name=  delete
POST   /servers/{id}/gameinfo/repair     re-add the Metamod search path
POST   /servers/{id}/metamod/toggle      {name, enabled}   rename the .vdf
GET    /servers/{id}/logs                tail of the newest CSS log
POST   /servers/{id}/restart             restart via servercontrol
GET    /servers/{id}/updates             latest upstream versions (?refresh=1)
GET    /servers/{id}/catalog             curated plugin catalog
POST   /servers/{id}/catalog/install     {key}             install from GitHub
POST   /servers/{id}/platform/install    {kind: metamod|css}
POST   /servers/{id}/snapshots           create snapshot
GET    /servers/{id}/snapshots           list snapshots
POST   /servers/{id}/snapshots/restore   {name}
DELETE /servers/{id}/snapshots           {name}
GET    /servers/{id}/audit               recent panel actions
POST   /servers/{id}/plugins/install-archive  {path, force?}  install uploaded zip
                                                              (32 MiB max, and no single
                                                               file inside it over that)
GET    /servers/{id}/doctor              server-side health checks
```

Beyond HTTP, the plugin exports two optional panel services: a
**ProtocolService** registering the CS2-tolerant Source RCON protocol for
game code `cs2` (transport: plugin, wire I/O over the `gameap-net` host
library), and a **ScheduledTaskHandler** with two tasks -
`cs2addons-gameinfo-autorepair` (6h) and `cs2addons-update-check` (24h).

The tab is shown only on Source 2 servers - gated on game code `cs2`, because
the panel's tab check matches engine or code and has no engine-version field,
while Source 1 games share the engine string `Source` - and requires the
`plugin:mnzteylemrxw4:manage` ability (granted to admins automatically). The
backend independently verifies engine `source` version `2` on every route. A
custom game entry with a different code needs adding to `codes` in
`frontend/src/index.ts`.

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
  game code is exactly `cs2` (the tab is gated on that code, so a custom or
  cloned game entry with a different code won't show it — add it to `codes` in
  `frontend/src/index.ts`); your user has admin rights (the
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
- **The plugin no longer loads after updating to 0.2.0** — the panel predates
  the `gameap-net`/`gameap-scheduler` host modules. Update the panel (or stay
  on a 0.1.x build of this plugin).
- **Platform install fails with EXTRACT_FAILED or DOWNLOAD_FAILED** — the
  node is missing `curl`/`unzip`; the error message names what was tried.
  `apt install curl unzip` and retry. Both features require a linux node.
- **No update badges appear** — versions come from api.github.com; check the
  panel's outbound plugin-HTTP policy (`PLUGIN_HTTP_*`) and the panel log.
  Unauthenticated GitHub API calls are also rate-limited per IP.
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

Tests: `cargo test` (89 backend tests run natively against the mock node — no
panel needed, including the tolerant-RCON engine over scripted byte streams)
and `cd frontend && npm test` (37 vitest tests for the parsers and status
logic).

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
