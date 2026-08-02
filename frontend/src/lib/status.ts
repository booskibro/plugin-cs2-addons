// Row status decision logic, kept pure so it can be unit-tested.

import type { RowStatus, RuntimePluginInfo } from '../types';

export interface RowStatusInput {
    enabled: boolean;
    missing: boolean;
    runtime: RuntimePluginInfo | null;
    rconOk: boolean;
}

export interface RowStatusResult {
    status: RowStatus;
    detail: string | null;
}

/**
 * Decides the row status from the folder state and the runtime
 * (`css_plugins list`) state.
 *
 * A runtime `stopped` (state UNLOADED — reached via `css_plugins stop` in the
 * server console) is a deliberate runtime state; it never becomes `pending`
 * and never asks for a restart.
 */
export function computeRowStatus({ enabled, missing, runtime, rconOk }: RowStatusInput): RowStatusResult {
    if (missing) {
        return { status: 'missing', detail: null };
    }
    if (runtime?.status === 'error') {
        return { status: 'error', detail: runtime.rawStatus };
    }
    if (!rconOk) {
        // Without console access there is no runtime to compare against.
        return { status: enabled ? 'enabled' : 'stopped', detail: null };
    }
    if (enabled && runtime?.status === 'running') {
        return { status: 'running', detail: null };
    }
    if (runtime?.status === 'stopped') {
        // Unloaded in the game — no restart needed, regardless of the folder.
        return { status: 'stopped', detail: runtime.rawStatus };
    }
    if (enabled && runtime === null) {
        // On disk but not loaded yet.
        return { status: 'pending', detail: null };
    }
    if (!enabled && runtime !== null) {
        // Disabled on disk but still in memory.
        return { status: 'pending', detail: null };
    }
    return { status: enabled ? 'enabled' : 'stopped', detail: null };
}

export function isPendingRow(status: RowStatus): boolean {
    return status === 'pending';
}

/**
 * Hot load/unload is a runtime action (`css_plugins load/stop` over RCON),
 * available only against live console state.
 */
export function hotActionForStatus(status: RowStatus, enabled: boolean): 'unload' | 'load' | null {
    if (status === 'running') return 'unload';
    if ((status === 'pending' || status === 'stopped') && enabled) return 'load';
    return null;
}
