// Client of the plugin's own WASM backend routes.

import axios from 'axios';

import { asHttpError, httpBodyMessage } from '../lib/http-error';
import type {
    AuditEntry,
    CatalogEntryInfo,
    CatalogInstallResult,
    LogsResponse,
    PlatformInstallResult,
    SnapshotCreateResult,
    SnapshotInfo,
    StateResponse,
    UpdatesResponse,
} from '../types';

function base(pluginId: string): string {
    return `/api/plugins/${pluginId}`;
}

export async function getState(pluginId: string, serverId: number): Promise<StateResponse> {
    const response = await axios.get(`${base(pluginId)}/servers/${serverId}/state`);
    return response.data as StateResponse;
}

export async function togglePlugin(
    pluginId: string,
    serverId: number,
    name: string,
    enabled: boolean,
): Promise<void> {
    await axios.post(`${base(pluginId)}/servers/${serverId}/plugins/toggle`, {
        name,
        enabled,
    });
}

export async function setAttributes(
    pluginId: string,
    serverId: number,
    name: string,
    comment: string | null,
    group: string | null,
): Promise<void> {
    await axios.post(`${base(pluginId)}/servers/${serverId}/plugins/attributes`, {
        name,
        comment,
        group,
    });
}

export async function registerPlugin(
    pluginId: string,
    serverId: number,
    payload: { name: string; force?: boolean },
): Promise<void> {
    await axios.post(`${base(pluginId)}/servers/${serverId}/plugins`, payload);
}

export async function deletePlugin(
    pluginId: string,
    serverId: number,
    name: string,
): Promise<void> {
    await axios.delete(`${base(pluginId)}/servers/${serverId}/plugins`, {
        data: { name },
        params: { name },
    });
}

export async function repairGameinfo(pluginId: string, serverId: number): Promise<boolean> {
    const response = await axios.post(`${base(pluginId)}/servers/${serverId}/gameinfo/repair`);
    return Boolean((response.data as { changed?: boolean }).changed);
}

export async function toggleMetamodPlugin(
    pluginId: string,
    serverId: number,
    name: string,
    enabled: boolean,
): Promise<void> {
    await axios.post(`${base(pluginId)}/servers/${serverId}/metamod/toggle`, { name, enabled });
}

export async function restartServer(pluginId: string, serverId: number): Promise<void> {
    await axios.post(`${base(pluginId)}/servers/${serverId}/restart`);
}

export async function getLogs(pluginId: string, serverId: number): Promise<LogsResponse> {
    const response = await axios.get(`${base(pluginId)}/servers/${serverId}/logs`);
    return response.data as LogsResponse;
}

export async function getUpdates(
    pluginId: string,
    serverId: number,
    refresh = false,
): Promise<UpdatesResponse> {
    const response = await axios.get(`${base(pluginId)}/servers/${serverId}/updates`, {
        params: refresh ? { refresh: 1 } : {},
    });
    return response.data as UpdatesResponse;
}

export async function getCatalog(
    pluginId: string,
    serverId: number,
): Promise<CatalogEntryInfo[]> {
    const response = await axios.get(`${base(pluginId)}/servers/${serverId}/catalog`);
    return (response.data as { entries: CatalogEntryInfo[] }).entries;
}

export async function installCatalogPlugin(
    pluginId: string,
    serverId: number,
    key: string,
): Promise<CatalogInstallResult> {
    const response = await axios.post(`${base(pluginId)}/servers/${serverId}/catalog/install`, {
        key,
    });
    return response.data as CatalogInstallResult;
}

export async function installPlatform(
    pluginId: string,
    serverId: number,
    kind: 'metamod' | 'css',
): Promise<PlatformInstallResult> {
    const response = await axios.post(`${base(pluginId)}/servers/${serverId}/platform/install`, {
        kind,
    });
    return response.data as PlatformInstallResult;
}

export async function createSnapshot(
    pluginId: string,
    serverId: number,
): Promise<SnapshotCreateResult> {
    const response = await axios.post(`${base(pluginId)}/servers/${serverId}/snapshots`);
    return response.data as SnapshotCreateResult;
}

export async function listSnapshots(pluginId: string, serverId: number): Promise<SnapshotInfo[]> {
    const response = await axios.get(`${base(pluginId)}/servers/${serverId}/snapshots`);
    return (response.data as { snapshots: SnapshotInfo[] }).snapshots;
}

export async function restoreSnapshot(
    pluginId: string,
    serverId: number,
    name: string,
): Promise<void> {
    await axios.post(`${base(pluginId)}/servers/${serverId}/snapshots/restore`, { name });
}

export async function deleteSnapshot(
    pluginId: string,
    serverId: number,
    name: string,
): Promise<void> {
    await axios.delete(`${base(pluginId)}/servers/${serverId}/snapshots`, { data: { name } });
}

export async function getAudit(pluginId: string, serverId: number): Promise<AuditEntry[]> {
    const response = await axios.get(`${base(pluginId)}/servers/${serverId}/audit`);
    return (response.data as { entries: AuditEntry[] }).entries;
}

/** Human-oriented message from a backend error response. */
export function apiErrorMessage(error: unknown, fallback: string): string {
    const http = asHttpError(error);
    if (http) {
        return httpBodyMessage(error) ?? http.message ?? fallback;
    }
    return fallback;
}
