import { describe, expect, it } from 'vitest';

import { computeRowStatus, hotActionForStatus } from '../lib/status';
import type { RuntimePluginInfo } from '../types';

function runtime(status: RuntimePluginInfo['status'], rawStatus = status): RuntimePluginInfo {
    return { name: 'MatchZy', version: '1.0', author: null, status, rawStatus };
}

describe('computeRowStatus', () => {
    it('missing wins over everything', () => {
        expect(
            computeRowStatus({ enabled: true, missing: true, runtime: runtime('running'), rconOk: true }),
        ).toEqual({ status: 'missing', detail: null });
    });

    it('runtime error is reported with detail', () => {
        expect(
            computeRowStatus({ enabled: true, missing: false, runtime: runtime('error', 'ERROR'), rconOk: true }),
        ).toEqual({ status: 'error', detail: 'ERROR' });
    });

    it('falls back to folder state without rcon', () => {
        expect(
            computeRowStatus({ enabled: true, missing: false, runtime: null, rconOk: false }),
        ).toEqual({ status: 'enabled', detail: null });
        expect(
            computeRowStatus({ enabled: false, missing: false, runtime: null, rconOk: false }),
        ).toEqual({ status: 'stopped', detail: null });
    });

    it('enabled and loaded is running', () => {
        expect(
            computeRowStatus({ enabled: true, missing: false, runtime: runtime('running'), rconOk: true }),
        ).toEqual({ status: 'running', detail: null });
    });

    it('deliberate runtime unload is stopped, not pending', () => {
        expect(
            computeRowStatus({
                enabled: true,
                missing: false,
                runtime: runtime('stopped', 'UNLOADED'),
                rconOk: true,
            }),
        ).toEqual({ status: 'stopped', detail: 'UNLOADED' });
    });

    it('enabled but not loaded is pending', () => {
        expect(
            computeRowStatus({ enabled: true, missing: false, runtime: null, rconOk: true }),
        ).toEqual({ status: 'pending', detail: null });
    });

    it('disabled but still in memory is pending', () => {
        expect(
            computeRowStatus({ enabled: false, missing: false, runtime: runtime('running'), rconOk: true }),
        ).toEqual({ status: 'pending', detail: null });
    });

    it('disabled and not loaded is stopped', () => {
        expect(
            computeRowStatus({ enabled: false, missing: false, runtime: null, rconOk: true }),
        ).toEqual({ status: 'stopped', detail: null });
    });
});

describe('hotActionForStatus', () => {
    it('running rows can be unloaded', () => {
        expect(hotActionForStatus('running', true)).toBe('unload');
    });

    it('enabled pending/stopped rows can be loaded', () => {
        expect(hotActionForStatus('pending', true)).toBe('load');
        expect(hotActionForStatus('stopped', true)).toBe('load');
    });

    it('disabled and broken rows get no hot action', () => {
        expect(hotActionForStatus('stopped', false)).toBeNull();
        expect(hotActionForStatus('missing', true)).toBeNull();
        expect(hotActionForStatus('error', true)).toBeNull();
    });
});
