// Client of the plugin's own WASM backend routes.

import axios from 'axios';

import type { StateResponse } from '../types';

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

/** Human-oriented message from a backend error response. */
export function apiErrorMessage(error: unknown, fallback: string): string {
    if (axios.isAxiosError(error)) {
        const data = error.response?.data as { message?: string } | undefined;
        if (data?.message) {
            return data.message;
        }
        return error.message;
    }
    return fallback;
}
