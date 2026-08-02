import { describe, expect, it } from 'vitest';

import {
    cssVersionFromMetaList,
    isBadPasswordOutput,
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

describe('isBadPasswordOutput', () => {
    it('detects bad password notices', () => {
        expect(isBadPasswordOutput('Bad Password')).toBe(true);
        expect(isBadPasswordOutput('bad rcon password')).toBe(true);
        expect(isBadPasswordOutput('map de_dust2')).toBe(false);
    });
});
