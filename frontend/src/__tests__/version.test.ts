import { describe, expect, it } from 'vitest';

import { versionKey, versionsMatch } from '../lib/version';

describe('versionKey', () => {
    it('strips prefixes, hashes and separators down to the numbers', () => {
        expect(versionKey('v1.0.371')).toBe('1.0.371');
        expect(versionKey('1.0.371 @ 3923c5d')).toBe('1.0.371');
        expect(versionKey('2.0.0-dev+1410')).toBe('2.0.0.1410');
        expect(versionKey('2.0.0-git1410')).toBe('2.0.0.1410');
    });
});

describe('versionsMatch', () => {
    it('treats differently-labeled identical builds as equal', () => {
        // The two false-badge field reports, verbatim.
        expect(versionsMatch('1.0.371 @ 3923c5d', '1.0.371')).toBe(true);
        expect(versionsMatch('2.0.0-dev+1410', '2.0.0-git1410')).toBe(true);
        expect(versionsMatch('v0.9.1', '0.9.1')).toBe(true);
    });

    it('still detects real updates', () => {
        expect(versionsMatch('1.0.371 @ 3923c5d', '1.0.372')).toBe(false);
        expect(versionsMatch('2.0.0-dev+1410', '2.0.0-git1411')).toBe(false);
        expect(versionsMatch('0.9.1', '0.10.0')).toBe(false);
    });

    it('falls back to plain comparison for number-free strings', () => {
        expect(versionsMatch('latest', 'Latest')).toBe(true);
        expect(versionsMatch('latest', 'stable')).toBe(false);
    });
});
