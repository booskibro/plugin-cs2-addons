import { describe, expect, it } from 'vitest';

import { fileExtension, fileStem, prettyName } from '../lib/naming';

describe('naming', () => {
    it('extracts extensions and stems', () => {
        expect(fileExtension('MatchZy.dll')).toBe('dll');
        expect(fileExtension('noext')).toBe('');
        expect(fileStem('MatchZy.dll')).toBe('MatchZy');
        expect(fileStem('noext')).toBe('noext');
    });

    it('prettifies folder names', () => {
        expect(prettyName('high_ping_kicker')).toBe('High Ping Kicker');
        expect(prettyName('WeaponPaints')).toBe('Weapon Paints');
        expect(prettyName('CS2-Tags')).toBe('CS2 Tags');
        expect(prettyName('MatchZy')).toBe('Match Zy');
    });
});
