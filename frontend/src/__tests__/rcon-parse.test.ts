import { describe, expect, it } from 'vitest';

import {
    cssVersionFromMetaList,
    isBadPasswordOutput,
    isUnknownCommandOutput,
    matchRuntimeToFolders,
    matchesModuleName,
    parseCssPlugins,
    parseMetaList,
    parseMetaVersion,
} from '../lib/rcon-parse';

describe('parseMetaVersion', () => {
    it('parses Metamod:Source 2.x output', () => {
        const output = [
            ' Metamod:Source Version Information',
            '    Metamod:Source version 2.0.0-dev+1359',
            '    Plugin interface version: 16:14',
            '    SourceHook version: 5:5',
            '    http://www.metamodsource.net/',
        ].join('\n');
        expect(parseMetaVersion(output)).toEqual({
            build: 'Metamod:Source',
            version: '2.0.0-dev+1359',
        });
    });

    it('returns null without a version line', () => {
        expect(parseMetaVersion('unknown command "meta"')).toBeNull();
        expect(parseMetaVersion('')).toBeNull();
    });
});

describe('parseMetaList / cssVersionFromMetaList', () => {
    const output = [
        'Listing 2 plugins:',
        '  [01] CounterStrikeSharp (v1.0.371) by Roflmuffin',
        '  [02] Some Other MM Plugin (2.4) by Someone Else',
    ].join('\n');

    it('parses meta list entries', () => {
        const entries = parseMetaList(output);
        expect(entries).toHaveLength(2);
        expect(entries[0]).toEqual({
            name: 'CounterStrikeSharp',
            version: '1.0.371',
            author: 'Roflmuffin',
        });
        expect(entries[1].name).toBe('Some Other MM Plugin');
    });

    it('extracts the CounterStrikeSharp core version', () => {
        expect(cssVersionFromMetaList(parseMetaList(output))).toEqual({
            build: 'CounterStrikeSharp',
            version: '1.0.371',
        });
        expect(cssVersionFromMetaList(parseMetaList('Listing 0 plugins:'))).toBeNull();
    });
});

describe('parseCssPlugins', () => {
    it('parses css_plugins list output', () => {
        const output = [
            '  List of all plugins currently loaded by CounterStrikeSharp: 3 plugins loaded.',
            '  [#1:LOADED]: "Addons Manager" (1.1.0) by BooskiBro',
            '  [#2:UNLOADED]: "MatchZy" (0.8.7) by WD-',
            '  [#3:ERROR]: "BrokenPlugin" (Unknown)',
        ].join('\n');
        const plugins = parseCssPlugins(output);
        expect(plugins).toHaveLength(3);
        expect(plugins[0]).toEqual({
            name: 'Addons Manager',
            version: '1.1.0',
            author: 'BooskiBro',
            status: 'running',
            rawStatus: 'LOADED',
        });
        expect(plugins[1].status).toBe('stopped');
        expect(plugins[2]).toMatchObject({
            name: 'BrokenPlugin',
            version: null,
            author: null,
            status: 'error',
        });
    });

    it('ignores the header and unrelated lines', () => {
        expect(parseCssPlugins('  List of all plugins currently loaded by CounterStrikeSharp: 0 plugins loaded.')).toEqual([]);
        expect(parseCssPlugins('server console noise')).toEqual([]);
    });
});

describe('matchesModuleName', () => {
    it('matches free-text module names to folder names', () => {
        expect(matchesModuleName('Addons Manager', 'AddonsManager')).toBe(true);
        expect(matchesModuleName('MatchZy', 'MatchZy')).toBe(true);
        expect(matchesModuleName('CS2 Tags', 'CS2-Tags')).toBe(true);
        expect(matchesModuleName('Weapon Paints', 'MatchZy')).toBe(false);
        expect(matchesModuleName('', 'MatchZy')).toBe(false);
    });
});

describe('matchRuntimeToFolders', () => {
    it('claims each runtime entry at most once', () => {
        const runtime = parseCssPlugins(
            [
                '  [#1:LOADED]: "MatchZy" (0.8.7) by WD-',
                '  [#2:LOADED]: "Weapon Paints" (3.1) by Nereziel',
            ].join('\n'),
        );
        const matched = matchRuntimeToFolders(['WeaponPaints', 'MatchZy', 'Missing'], runtime);
        expect(matched[0]?.name).toBe('Weapon Paints');
        expect(matched[1]?.name).toBe('MatchZy');
        expect(matched[2]).toBeNull();
    });
});

// Verbatim `css_plugins list` output from a real server, kept as-is: every
// mismatch below was reported from the tab against exactly these lines.
const REAL_LIST = [
    'List of all plugins currently loaded by CounterStrikeSharp: 6 plugins loaded.',
    '[#1:LOADED]: "[CS2-SimpleAdmin] Stealth Module" (v1.0.2) by daffyy',
    '[#2:LOADED]: "CS2-SimpleAdmin Fun Commands" (1.0.0) by Your Name',
    '   Fun commands extension for CS2-SimpleAdmin',
    '[#3:LOADED]: "RockTheVote" (1.9.6 (RELEASE)) by abnerfs, Oz-Lin',
    '   https://github.com/oz-lin/cs2-rockthevote',
    '[#4:UNLOADED]: "CS2-SimpleAdmin (RELEASE)" (1.7.8-beta-10b) by daffyy',
    '   Simple admin plugin for Counter-Strike 2 :)',
    '[#5:LOADED]: "Connect Disconnect Sound (Continent , Country , City , Message , Sounds , Logs , Discord)" (1.1.7) by Gold KingZ',
    '   https://github.com/oqyh',
    '[#6:LOADED]: "PlayerSettings [Core]" (0.9.4) by Nick Fox',
    "   One storage for player's settings (aka ClientCookies)",
].join('\n');

describe('real server output', () => {
    it('parses every entry, including a version containing parentheses', () => {
        const runtime = parseCssPlugins(REAL_LIST);
        expect(runtime).toHaveLength(6);
        const rtv = runtime.find((entry) => entry.name === 'RockTheVote');
        expect(rtv?.status).toBe('running');
        expect(rtv?.version).toBe('1.9.6');
        expect(rtv?.author).toBe('abnerfs, Oz-Lin');
    });

    it('pairs decorated module names with their folders', () => {
        const runtime = parseCssPlugins(REAL_LIST);
        const folders = [
            'CS2-SimpleAdmin',
            'CS2-SimpleAdmin_FunCommands',
            'CS2-SimpleAdmin_StealthModule',
            'Connect-Disconnect-Sound-GoldKingZ',
            'MenuManagerCore',
            'PlayerSettings',
            'RockTheVote',
        ];
        const matched = matchRuntimeToFolders(folders, runtime);
        const nameFor = (folder: string) => matched[folders.indexOf(folder)]?.name ?? null;

        // Exact matches must win before any looser rule runs, or the bare
        // CS2-SimpleAdmin folder swallows one of its own extensions.
        expect(nameFor('CS2-SimpleAdmin_FunCommands')).toBe('CS2-SimpleAdmin Fun Commands');
        expect(nameFor('CS2-SimpleAdmin_StealthModule')).toBe('[CS2-SimpleAdmin] Stealth Module');
        expect(nameFor('RockTheVote')).toBe('RockTheVote');

        // Decorated names, matched by the looser passes.
        expect(nameFor('CS2-SimpleAdmin')).toBe('CS2-SimpleAdmin (RELEASE)');
        expect(nameFor('PlayerSettings')).toBe('PlayerSettings [Core]');
        expect(nameFor('Connect-Disconnect-Sound-GoldKingZ')).toContain('Connect Disconnect Sound');

        // Genuinely not loaded - it must not borrow a leftover entry.
        expect(nameFor('MenuManagerCore')).toBeNull();
    });

    it('reports the right load states', () => {
        const runtime = parseCssPlugins(REAL_LIST);
        const folders = ['CS2-SimpleAdmin', 'RockTheVote'];
        const matched = matchRuntimeToFolders(folders, runtime);
        // UNLOADED in the console is a deliberate stop, not "awaiting load".
        expect(matched[0]?.status).toBe('stopped');
        expect(matched[1]?.status).toBe('running');
    });
});

describe('isUnknownCommandOutput', () => {
    it('detects an unknown command reply', () => {
        expect(isUnknownCommandOutput("Unknown command 'css_plugins'")).toBe(true);
        expect(isUnknownCommandOutput('unknown command: meta')).toBe(true);
        expect(isUnknownCommandOutput('  [#1:LOADED]: "MatchZy" (0.8.7) by WD-')).toBe(false);
        expect(isUnknownCommandOutput('')).toBe(false);
    });
});

describe('isBadPasswordOutput', () => {
    it('detects bad password notices', () => {
        expect(isBadPasswordOutput('Bad Password')).toBe(true);
        expect(isBadPasswordOutput('bad rcon password')).toBe(true);
        expect(isBadPasswordOutput('map de_dust2')).toBe(false);
    });
});
