// Existing GameAP panel endpoints the plugin frontend uses directly:
// RCON for versions/runtime status, file-manager for uploads and configs.

import axios from 'axios';

import { isBadPasswordOutput } from '../lib/rcon-parse';

export type RconFailure = 'offline' | 'no-rcon' | 'bad-password' | 'empty' | 'error';

export class RconError extends Error {
    reason: RconFailure;

    constructor(reason: RconFailure, message: string) {
        super(message);
        this.reason = reason;
    }
}

/** POST /api/servers/{id}/rcon — synchronous command output. */
export async function rcon(
    serverId: number,
    command: string,
    options?: { allowEmpty?: boolean },
): Promise<string> {
    try {
        const response = await axios.post(`/api/servers/${serverId}/rcon`, { command });
        const output = String(response.data?.output ?? '');
        if (isBadPasswordOutput(output)) {
            throw new RconError('bad-password', 'wrong rcon password');
        }
        if (output.trim() === '' && !options?.allowEmpty) {
            throw new RconError('empty', 'empty rcon output');
        }
        return output;
    } catch (error) {
        throw toRconError(error);
    }
}

/**
 * `css_plugins load/stop` — hot load or unload a plugin without a restart.
 * Empty output is the normal success case.
 */
export async function cssPluginsCommand(
    serverId: number,
    action: 'load' | 'stop',
    argument: string,
): Promise<string> {
    return rcon(serverId, `css_plugins ${action} "${argument}"`, { allowEmpty: true });
}

function toRconError(error: unknown): RconError {
    if (error instanceof RconError) {
        return error;
    }
    const status = axios.isAxiosError(error) ? error.response?.status : undefined;
    if (status === 503) {
        return new RconError('offline', 'server is offline');
    }
    if (status === 412) {
        return new RconError('no-rcon', 'rcon password is not configured');
    }
    if (status === 422) {
        return new RconError('bad-password', 'rcon authentication failed');
    }
    if (axios.isAxiosError(error)) {
        // The panel body's "message" says what actually broke (e.g. "failed to
        // execute rcon command: response ID mismatch"); the axios message is
        // only the HTTP status line.
        const data = error.response?.data as { message?: string } | undefined;
        return new RconError('error', data?.message ?? error.message);
    }
    return new RconError('error', String(error));
}

/** POST /api/file-manager/{id}/update-file — multipart write into a directory. */
export async function fmUploadFile(
    serverId: number,
    directory: string,
    file: File,
    onProgress?: (percent: number) => void,
): Promise<void> {
    const form = new FormData();
    form.append('disk', 'server');
    form.append('path', directory);
    form.append('file', file);
    await axios.post(`/api/file-manager/${serverId}/update-file`, form, {
        onUploadProgress: (event) => {
            if (onProgress && event.total) {
                onProgress(Math.round((event.loaded / event.total) * 100));
            }
        },
    });
}

/** GET /api/file-manager/{id}/download — raw file content as text. */
export async function fmDownloadText(serverId: number, path: string): Promise<string> {
    const response = await axios.get(`/api/file-manager/${serverId}/download`, {
        params: { disk: 'server', path },
        responseType: 'text',
        transformResponse: [(data: unknown) => data],
    });
    return String(response.data ?? '');
}

/** POST /api/file-manager/{id}/create-directory — ignores "already exists". */
export async function fmEnsureDirectory(
    serverId: number,
    parent: string,
    name: string,
): Promise<void> {
    try {
        await axios.post(`/api/file-manager/${serverId}/create-directory`, {
            disk: 'server',
            path: parent,
            name,
        });
    } catch (error) {
        if (axios.isAxiosError(error) && error.response && error.response.status < 500) {
            return; // directory already exists
        }
        throw error;
    }
}
