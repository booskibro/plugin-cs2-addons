// Parsers of Source 2 console command output obtained via RCON.

import type { PlatformVersion, RuntimePluginInfo } from '../types';

/**
 * `meta version` (Metamod:Source 2.x) →
 *   Metamod:Source Version Information
 *      Metamod:Source version 2.0.0-dev+1359
 *      ...
 */
export function parseMetaVersion(output: string): PlatformVersion | null {
    const match = /Metamod:Source\s+version\s+v?([0-9][\w.+-]*)/i.exec(output);
    if (!match) {
        return null;
    }
    return { build: 'Metamod:Source', version: match[1].replace(/[.,]+$/, '') };
}

/**
 * `meta list` (Metamod:Source 2.x) →
 *   Listing 1 plugin:
 *     [01] CounterStrikeSharp (v1.0.371) by Roflmuffin
 */
const META_LIST_LINE =
    /^\s*\[\s*(\d+)\]\s+(?<name>.+?)\s+\(v?(?<vers>[^)]*)\)(?:\s+by\s+(?<author>.+))?$/;

export interface MetaListEntry {
    name: string;
    version: string | null;
    author: string | null;
}

export function parseMetaList(output: string): MetaListEntry[] {
    const result: MetaListEntry[] = [];
    for (const line of output.split('\n')) {
        const match = META_LIST_LINE.exec(line.trimEnd());
        if (!match?.groups) {
            continue;
        }
        const { name, vers, author } = match.groups;
        result.push({
            name: name.trim(),
            version: normalizeVersion(vers),
            author: author?.trim() || null,
        });
    }
    return result;
}

/** CounterStrikeSharp core version from the `meta list` entry. */
export function cssVersionFromMetaList(entries: MetaListEntry[]): PlatformVersion | null {
    const css = entries.find((entry) => /counterstrikesharp/i.test(entry.name));
    if (!css) {
        return null;
    }
    return { build: 'CounterStrikeSharp', version: css.version ?? '' };
}

/**
 * `css_plugins list` →
 *     List of all plugins currently loaded by CounterStrikeSharp: 2 plugins loaded.
 *     [#1:LOADED]: "Addons Manager" (1.1.0) by BooskiBro
 *     [#2:UNLOADED]: "MatchZy" (0.8.7) by WD-
 *
 * (see Application.cs in roflmuffin/CounterStrikeSharp: the line is
 *  `  [#{id}:{STATE}]: "{ModuleName}" ({ModuleVersion})` + optional ` by {Author}`)
 */
// The version group is greedy rather than [^)]*: plugins ship versions that
// contain parentheses ("1.9.6 (RELEASE)"), and a non-greedy class stopped at
// the inner ")" and failed the whole line - dropping a loaded plugin from the
// runtime list entirely, so its row claimed to be awaiting load.
const CSS_PLUGINS_LINE =
    /^\s*\[#(?<id>\d+):(?<state>[A-Za-z]+)\]:\s*"(?<name>.*)"\s+\((?<vers>.*)\)(?:\s+by\s+(?<author>.+))?$/;

export function parseCssPlugins(output: string): RuntimePluginInfo[] {
    const result: RuntimePluginInfo[] = [];
    for (const line of output.split('\n')) {
        const match = CSS_PLUGINS_LINE.exec(line.trimEnd());
        if (!match?.groups) {
            continue;
        }
        const { state, name, vers, author } = match.groups;
        result.push({
            name: name.trim(),
            version: normalizeVersion(vers),
            author: author?.trim() || null,
            status: normalizeCssStatus(state),
            rawStatus: state,
        });
    }
    return result;
}

/**
 * Matches a runtime ModuleName against a plugin folder name. ModuleName is
 * free text ("Addons Manager") while the folder is its compact form
 * ("AddonsManager") — compare with everything but letters/digits stripped.
 */
export function matchesModuleName(moduleName: string, folder: string): boolean {
    const normalizedModule = normalizeKey(moduleName);
    const normalizedFolder = normalizeKey(folder);
    if (!normalizedModule || !normalizedFolder) {
        return false;
    }
    return normalizedModule === normalizedFolder;
}

function normalizeKey(value: string): string {
    return value.toLowerCase().replace(/[^a-z0-9]/g, '');
}

/**
 * Pairs plugin folder names with runtime entries from `css_plugins list`.
 * Each runtime entry is consumed at most once so duplicate module names keep
 * one-to-one alignment instead of every row matching the first entry.
 */
export function matchRuntimeToFolders(
    folders: string[],
    runtimeList: RuntimePluginInfo[],
): (RuntimePluginInfo | null)[] {
    const folderKeys = folders.map(normalizeKey);
    const moduleKeys = runtimeList.map((item) => normalizeKey(item.name));
    const matched: (RuntimePluginInfo | null)[] = folders.map(() => null);
    const claimed = new Set<number>();

    // Strictest rule first, and every pass runs over all folders before the
    // next one starts. Ordering is the whole trick: it stops a loose rule from
    // stealing an entry that some other folder matches exactly - "CS2-SimpleAdmin"
    // is a prefix of "CS2-SimpleAdmin Fun Commands", so a single loose pass
    // would bind it to the wrong plugin.
    for (const accepts of [exactKey, prefixKey, sharedPrefixKey]) {
        folderKeys.forEach((folderKey, folderIndex) => {
            if (matched[folderIndex] !== null || folderKey === '') {
                return;
            }
            // Closest candidate, not the first acceptable one: for folder
            // CS2-SimpleAdmin the prefix rule accepts "CS2-SimpleAdmin (RELEASE)",
            // "CS2-SimpleAdmin Fun Commands" and "[CS2-SimpleAdmin] Stealth
            // Module" alike, and only the least-decorated one is the plugin.
            let best = -1;
            let bestDistance = Number.POSITIVE_INFINITY;
            moduleKeys.forEach((moduleKey, i) => {
                if (claimed.has(i) || moduleKey === '' || !accepts(moduleKey, folderKey)) {
                    return;
                }
                const distance = Math.abs(moduleKey.length - folderKey.length);
                if (distance < bestDistance) {
                    bestDistance = distance;
                    best = i;
                }
            });
            if (best < 0) {
                return;
            }
            claimed.add(best);
            matched[folderIndex] = runtimeList[best];
        });
    }
    return matched;
}

function exactKey(moduleKey: string, folderKey: string): boolean {
    return moduleKey === folderKey;
}

/** "PlayerSettings [Core]" for folder PlayerSettings, "CS2-SimpleAdmin (RELEASE)"
 * for CS2-SimpleAdmin: authors decorate ModuleName with suffixes. */
function prefixKey(moduleKey: string, folderKey: string): boolean {
    return moduleKey.startsWith(folderKey) || folderKey.startsWith(moduleKey);
}

/** Last resort, for a ModuleName that is really a description
 * ("Connect Disconnect Sound (Continent , Country , …)" for folder
 * Connect-Disconnect-Sound-GoldKingZ): accept a long shared opening that
 * covers most of the folder name. A heuristic, so it only ever sees entries
 * the two stricter passes left unclaimed. */
const MIN_SHARED_PREFIX = 10;
const SHARED_PREFIX_RATIO = 2 / 3;

function sharedPrefixKey(moduleKey: string, folderKey: string): boolean {
    let shared = 0;
    while (
        shared < moduleKey.length &&
        shared < folderKey.length &&
        moduleKey[shared] === folderKey[shared]
    ) {
        shared += 1;
    }
    return shared >= MIN_SHARED_PREFIX && shared >= folderKey.length * SHARED_PREFIX_RATIO;
}

/**
 * `Unknown command 'css_plugins'` - the console does not have that command,
 * which for a `css_` command means CounterStrikeSharp is not loaded in the
 * running server (its files on disk say nothing about that). Older CS2 builds
 * answered unknown commands with silence instead, so an empty reply is not
 * proof of the opposite.
 */
export function isUnknownCommandOutput(output: string): boolean {
    return /unknown\s+command/i.test(output);
}

/** Source 2 rcon answers with a bad-password notice on auth failure. */
export function isBadPasswordOutput(output: string): boolean {
    return /bad\s*(rcon\s*)?password/i.test(output);
}

function normalizeVersion(raw: string): string | null {
    // "1.9.6 (RELEASE)" → "1.9.6": the decoration is noise in the column and
    // would never compare equal to a release tag when checking for updates.
    const cleaned = raw.replace(/^v/i, '').trim().split(/\s+/)[0] ?? '';
    return /^[0-9]/.test(cleaned) ? cleaned : null;
}

function normalizeCssStatus(state: string): RuntimePluginInfo['status'] {
    const lowered = state.toLowerCase();
    if (lowered === 'loaded') {
        return 'running';
    }
    if (lowered === 'unloaded' || lowered === 'unregistered') {
        return 'stopped';
    }
    return 'error'; // ERROR and anything unexpected
}
