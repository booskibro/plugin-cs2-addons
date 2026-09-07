# Changelog

Versions are the plugin's own, as reported in `PluginInfo` and shown in the
panel's plugin list.

**v0.6.4 was the first GitHub release; 1.0.0 is the first published on
plugins.gameap.dev.** Everything below it was developed
and built, but never published as a GitHub release: 0.1.0–0.3.4 exist as
commits in this repo, while 0.3.5–0.6.3 were working builds that reached a
live server without being tagged, and landed here folded into the two commits
that make up v0.6.4. Their entries are kept because the behaviour they
describe is in the shipped plugin — not because those artifacts are available.

## 1.0.0 — first public release *(released)*

Published on **plugins.gameap.dev**. The store move was meant to be all this
was, and four release candidates on a live server turned it into the release
where CounterStrikeSharp actually loads.

### CounterStrikeSharp would not load

The whole of it: CSS was installed correctly, `meta list` reported it as
`<ERROR>`, every `css_` console command was gone, and reinstalling fetched the
same broken file. Four candidates chasing one cause.

- **Fix executable stack.** CounterStrikeSharp ships releases whose
  `counterstrikesharp.so` marks `PT_GNU_STACK` executable. Current kernels
  refuse that on `dlopen`, so the platform fails to load while every file on
  disk looks perfect. The documented remedies are `execstack -c` and
  `patchelf --clear-execstack`; `execstack` shipped in `prelink`, which recent
  distributions dropped, so the fix is missing on exactly the systems new
  enough to need it. The plugin locates the four-byte `p_flags` field itself
  and clears `PF_X` on the node.
- **Four bytes, not nine megabytes.** The first two attempts at that write
  failed, and rc.2's *request cancelled* was misread as the panel abandoning
  the request on navigation. It was not: uploading the 9.7MB patched library
  severed the daemon's gRPC session, which the panel log named at the exact
  second of all three attempts. Only the four changed bytes travel now — up as
  a scratch file, into place with `dd conv=notrunc`, read back with `od`.
- **The read-back needed its own fix.** On the first live run the repair landed
  and still reported failure: the check scanned od's output for anything
  parseable as hex and took the `4` out of its own `-N 4`. It now accepts
  exactly four two-digit hex tokens, so a surprise skips the check rather than
  calling a good write corrupt.
- **The Doctor asks Metamod** why a platform failed to load rather than
  inferring it from the files, and reports what Metamod says.
- **Reinstall** for a platform that is installed but not loading — previously
  the only platform button was Install, which an installed platform never
  showed, so there was no way to replace a bad file from the tab.
- **Install latest** returns for a `counterstrikesharp/` directory that exists
  but is empty, which a failed install leaves behind.

### Writes to the node have a real limit now

The same mistaken assumption was gating archive installs.

- **A single file inside an uploaded zip** was checked against the 32MB read
  limit, far above what the daemon will accept on one write, so an archive
  holding one large `.dll` would have found this the hard way. The gate is now
  `MAX_NODE_WRITE_BYTES`, just under 4MiB.
- **`PANEL_MAX_INLINE_BYTES` keeps its number and its job** bounding reads, and
  its comment no longer describes a panel setting that could not be found in
  the source. What is documented there now is what was actually verified.
- **Long writes are not cancelled by navigating away.** The panel's axios
  instance aborts in-flight requests of a route on navigation unless the caller
  supplies its own signal. Reads should be cancelled; a write that is abandoned
  client-side while it continues server-side should not, so the writes now
  carry a signal that is never fired.

### The store move

- **The plugin id changed** from `mnzteylemrxw4` — hand-picked as
  base32("cs2addon") back when it only had to be unique on one panel — to
  **`kgebvqj6rsfh4`**, assigned by the store and fixed for the life of the
  listing. To a panel this makes it a *different plugin*: routes move to
  `/api/plugins/kgebvqj6rsfh4`, the ability becomes
  `plugin:kgebvqj6rsfh4:manage`, and per-plugin storage starts empty, so audit
  history and the cached update checks do not carry over. Snapshots are files
  on the game server and are unaffected. Anyone running a pre-1.0.0 build
  should uninstall it before installing this one, or the panel will show two
  Plugins tabs.
- **Releases publish themselves to the store.** `release.yml` uploads each
  GitHub release: version from the tag, changelog from the release body,
  `is_stable` from whether the release is a prerelease. Signing is optional —
  with a `GPG_SIGNING_KEY` secret the upload carries a detached signature,
  without one it goes unsigned, which the store accepts. The store step is
  skipped entirely until the `GAMEAP_PLUGIN_ID` variable is set.
- **`min_gameap_version` is declared as 4.4.0** — the release where the
  `gameap-net` and `gameap-scheduler` host modules first shipped (both absent
  in 4.3.5), without which the plugin cannot load at all. The 4.5-only pieces
  degrade rather than break: an older panel ignores the declared
  `required_permissions`, and has no nodefs inline cap for the archive guard to
  run into.
- The README's RCON requirement now says GameAP generates the password when a
  server is created, instead of implying one has to be set by hand.

## 0.6.4 — GameAP 4.5 *(released)*

Also covers 0.6.3; the two were developed together and the boundary between
them was not recorded. Panel-facing rather than tab-facing.

- `PluginInfo` declares `required_permissions`: `node_commands`, `files`,
  `manage_servers` — each the narrowest grant covering a host call the plugin
  actually makes. `files_read` is deliberately absent, since `files` covers it
  and declaring both disagrees with the panel's own derivation from the wasm
  import section. Nothing is enforced yet (`PLUGINS_PERMISSIONS_ENFORCE`
  defaults to false, and older panels ignore the field), but a later release
  turns it on and an undeclared plugin is refused then.
- Archive install refuses an entry over the panel's `PLUGINS_NODEFS_MAX_INLINE`
  cap before writing a single byte. Entries upload one at a time, so finding an
  oversized one halfway through would leave a half-installed plugin. Tested at
  exactly the limit as well as one byte over, because the panel's own
  comparison is strict.
- The log reader documents the same ceiling: it downloads the whole file and
  keeps the tail, so a log grown past the limit loses the logs route and the
  doctor route with it.

## 0.6.2 — Source 2 only *(never released)*

- The **Plugins** tab appeared on Source 1 servers (CS:S, TF2, L4D2), where
  every route returns 422. It is now gated on game code `cs2`. It has to be by
  code: Source 1 and Source 2 share the engine string `Source`, the panel's
  `GameCheck` has no engine-version field, and its matcher ORs engines with
  codes — so naming the engine at all would let Source 1 back in.

## 0.6.1 — runtime matching *(never released)*

Two fixes to how rows are paired with `css_plugins list` output.

- A version containing parentheses (`"RockTheVote" (1.9.6 (RELEASE))`) failed
  the line regex, dropping that plugin from the runtime list entirely — so a
  loaded plugin showed as *Awaiting load*.
- ModuleName is matched to the folder name in three passes — exact, then
  prefix, then a long shared opening — each running over every row before the
  next begins. Decorated names like `PlayerSettings [Core]` and
  `CS2-SimpleAdmin (RELEASE)` now pair with their folders, while
  `CS2-SimpleAdmin` cannot steal the entry belonging to
  `CS2-SimpleAdmin_FunCommands`. Within a pass the closest candidate by name
  length wins, so the least-decorated match is preferred.

## 0.6.0 — placement *(never released)*

- **Fixed:** a release zip shipping its plugin as a bare `<Name>/` folder
  beside `shared/` unpacked one level too high, into
  `addons/counterstrikesharp/`, where the dotnet host fails with *Failed to
  locate managed application*. Archive entries are now routed per top-level
  folder: `plugins/`, `shared/`, `configs/` and `gamedata/` keep their prefix,
  everything else lands in `plugins/`. Introduced in 0.4.0.
- The Doctor gains a placement check for plugin folders sitting outside
  `plugins/`, and its shared-assembly check names the dll a folder actually
  holds (`shared/GoldKingZ/ holds GoldKingZ.Api.dll — rename the folder to
  GoldKingZ.Api`) rather than only reporting one missing.
- The restart banner stops crying wolf: it no longer counts rows that are
  enabled on disk but not loaded, a state a plugin can sit in permanently when
  it fails to load or its module name never matches its folder.

## 0.5.0 — platform safety *(never released)*

- `addons/metamod/counterstrikesharp.vdf` appears in the Metamod plugin list
  like any other binary plugin, but switching it off unloads the whole
  platform. It now carries a *platform* badge, asks for confirmation, and the
  backend refuses the toggle without an explicit `force`.
- The console's `unknown command` reply is recognised as CounterStrikeSharp not
  being loaded — reported in the hint line, in a failed Load's toast and as a
  Doctor check — instead of being parsed as "no plugins running" and quietly
  showing folder state.

## 0.4.1 — toolbar labels *(never released)*

- The toolbar icons carry their labels as hover tooltips. The labels had never
  rendered at any width: the panel's Tailwind build emits no `sm:inline`, so
  the `hidden sm:inline` spans were hidden everywhere.

## 0.4.0 — shared assemblies and load diagnostics *(never released)*

- **Shared-assembly support.** Release zips rooted at `plugins/` + `shared/`
  were rejected outright, so a plugin's contract assembly never got installed
  and the plugin threw at load. Those layouts install now, `shared/` is
  captured in snapshots, and the Doctor checks its layout. This matters beyond
  the one plugin: a plugin that throws during load leaves CounterStrikeSharp
  holding a context with no instance, and every later `css_plugins load` then
  fails on it.
- **Load-failure check.** The Doctor reads the CounterStrikeSharp log and names
  any plugin that failed to load, plus the assembly it could not find.
- **Failed hot actions explain themselves.** CSS logs load errors rather than
  answering on the console, so a failed *Load* surfaces the log's last error
  line instead of a red toast with no reason.
- The admins editor became an editable table, and the Doctor dialog lays out in
  two columns.

## 0.3.5 — metadata *(never released)*

- Author string aligned with the other GameAP plugins from the same author.
  Credit for the original remains in the README.

## 0.3.4 — version comparison and hot-load timing

- Update badges compared version strings literally, so identical builds in
  different costumes (`1.0.371 @ 3923c5d` vs `1.0.371`) produced false update
  offers. Versions now compare by their digit-sequence key.
- Load/Unload/Reload re-listed `css_plugins` immediately, racing CS2's async
  plugin loading and misreporting success as failure; the re-list waits 800ms.
- A whitespace-only command output rendered as an empty red toast.

## 0.3.3 — RCON response delimiting

- Dropped the pipelined end-marker. This CS2 build discards a pending command's
  output when a second request arrives behind it, so `execute()` now sends
  exactly one request per command and delimits the response by idle gap. Total
  silence became an empty output rather than an error.

## 0.3.2 — RCON request ids

- Command output arriving under a foreign request id is accepted rather than
  discarded; empty exchanges are logged.

## 0.3.1 — RCON marker echo

- Tolerates CS2's marker echo overtaking command output, and explains an empty
  `meta version` instead of reporting a generic failure.

## 0.3.0 — zip installs, snapshots, Doctor

- **Zip installs:** any release archive dropped on the Upload dialog; the
  layout is detected and unpacked to the right place, folders registered
  automatically. 409 plus confirm-retry on conflicts.
- **Automatic safety snapshots** before platform installs, catalog installs,
  zip installs and restores, so every overwrite is reversible.
- **-usercon detector:** when RCON fails and the launch command lacks
  `-usercon`, the hint says exactly that.
- **Doctor dialog** running every health check in one pass.
- An **Update all** button, a per-row **Reload** action, JSON validation and a
  Format button in the config editor, and log following plus a download link in
  the log viewer.

## 0.2.1 — axios instance

- The panel externalizes axios as an *instance*, which carries none of the
  namespace statics, so every `axios.isAxiosError()` call threw at runtime and
  replaced the real error before it could be classified. Error inspection is
  now duck-typed. This is why RCON failures had only ever shown the generic
  bucket with no reason since 0.1.0.

## 0.2.0 — installs, RCON, catalog, snapshots

- One-click Metamod:Source and CounterStrikeSharp install and update, unpacked
  on the node.
- `gameinfo.gi` **Fix** button plus a 6-hour scheduled sweep that re-wires every
  CS2 server a game update reverted.
- A **CS2-tolerant Source RCON** protocol registered over `gameap-net`:
  multi-packet reassembly, oversized packets, both auth-response shapes, and
  unsolicited console output skipped.
- Curated plugin **catalog** installed from GitHub releases, with nightly update
  checks and update badges.
- Metamod `.vdf` toggles, the CSS admins editor, the log-tail modal, the restart
  banner, **snapshots** with restore, and an **audit history** of panel actions.

## 0.1.0 — initial

- Port of [gameap/plugin-goldsrc-addons](https://github.com/gameap/plugin-goldsrc-addons)
  to Counter-Strike 2: Rust/WASM backend on the gameap-plugin-sdk with an
  embedded Vue 3 frontend. `plugins.ini` semantics map to plugin folder moves
  (`plugins/disabled`) and a `plugins_meta.json` manifest shared with the
  cs2-addons-manager server plugin; live statuses come from RCON.

Two fixes followed before 0.2.0: the plugin id became `mnzteylemrxw4`, because
the panel normalizes ids through a base32 round-trip and `cs2addons` is an
invalid length — it fell back to an FNV hash, and the granted ability then never
matched the tab gate, hiding the tab entirely. And enable/disable moved to a
native nodefs move: the daemon shellquote-splits `nodecmd` commands and execs
them directly, so there is no shell to run `mv` in.
