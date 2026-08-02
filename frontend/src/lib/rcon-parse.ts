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
const CSS_PLUGINS_LINE =
    /^\s*\[#(?<id>\d+):(?<state>[A-Za-z]+)\]:\s*"(?<name>.*)"\s+\((?<vers>[^)]*)\)(?:\s+by\s+(?<author>.+))?$/;

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
    const claimed = new Set<number>();
    return folders.map((folder) => {
        const index = runtimeList.findIndex(
            (item, i) => !claimed.has(i) && matchesModuleName(item.name, folder),
        );
        if (index < 0) {
            return null;
        }
        claimed.add(index);
        return runtimeList[index];
    });
}

/** Source 2 rcon answers with a bad-password notice on auth failure. */
export function isBadPasswordOutput(output: string): boolean {
    return /bad\s*(rcon\s*)?password/i.test(output);
}

function normalizeVersion(raw: string): string | null {
    const cleaned = raw.replace(/^v/i, '').trim();
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
