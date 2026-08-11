<template>
    <div>
        <!-- Fallback guard: the tab itself is game-gated via checkGame -->
        <n-card v-if="!isSource2" size="small">
            <div class="py-10">
                <n-empty :description="trans('not_source2')" size="small" />
            </div>
        </n-card>

        <template v-else>
            <Loading v-if="loading && !state" />

            <n-card v-else-if="loadError" size="small">
                <div class="py-8 text-center">
                    <div class="text-sm text-red-500 dark:text-red-400 mb-3">{{ loadError }}</div>
                    <GButton color="white" size="small" @click="refreshAll">
                        <GIcon name="refresh" /><span class="ml-1">{{ trans('retry') }}</span>
                    </GButton>
                </div>
            </n-card>

            <template v-else-if="state">
                <!-- rcon hint: always rendered to reserve its height — removing it
                     from the layout shifts the content below (visibility keeps the
                     box; nbsp keeps the text line height when there is no hint) -->
                <div
                    class="mb-1 flex items-center gap-2 text-xs text-stone-400 dark:text-stone-500"
                    :style="{ visibility: rconHint ? 'visible' : 'hidden' }"
                >
                    <GIcon name="info" size="sm" />
                    <span>{{ rconHint || ' ' }}</span>
                </div>

                <!-- platform status cards; only CounterStrikeSharp carries the list -->
                <div class="platform-cards grid md:grid-cols-2 gap-3 mb-3">
                    <PlatformCard
                        kind="metamod"
                        :state="state"
                        :version="metaVersion"
                        :rows="[]"
                    />
                    <PlatformCard
                        kind="css"
                        :state="state"
                        :version="cssVersion"
                        :rows="rows"
                        active
                    />
                </div>

                <!-- plugin list -->
                <n-card size="small" class="plugins-panel">
                    <template v-if="nothingInstalled">
                        <div class="py-12 text-center">
                            <i class="fa-solid fa-puzzle-piece fa-2x text-stone-300 dark:text-stone-600"></i>
                            <div class="mt-3 font-medium text-stone-700 dark:text-stone-200">
                                {{ trans('nothing_installed_title') }}
                            </div>
                            <div class="mt-1 text-sm text-stone-500 dark:text-stone-400 max-w-md mx-auto">
                                {{ trans('nothing_installed_text') }}
                            </div>
                        </div>
                    </template>

                    <template v-else>
                        <PluginList
                            :rows="rows"
                            :installed="state.css.installed"
                            :plugins-path="state.paths.css_plugins_dir"
                            :busy="mutating"
                            @toggle="onToggle"
                            @hot-action="onHotAction"
                            @set-comment="onSetComment"
                            @remove="onDelete"
                            @bulk="onBulk"
                            @install="installOpen = true"
                            @configure="openConfig"
                            @open-files="openFileManager"
                        />
                    </template>
                </n-card>

                <InstallModal
                    v-model:show="installOpen"
                    :server-id="serverId"
                    :plugin-id="pluginId"
                    :paths="state.paths"
                    :existing-names="rows.map((row) => row.name)"
                    @installed="onInstalled"
                />
                <ConfigModal
                    v-model:show="configOpen"
                    :server-id="serverId"
                    :row="configRow"
                />
            </template>
        </template>
    </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { NCard, NEmpty } from 'naive-ui';
import { providePluginTrans } from '@gameap/plugin-sdk';

import ConfigModal from './ConfigModal.vue';
import InstallModal from './InstallModal.vue';
import PlatformCard from './PlatformCard.vue';
import PluginList from './PluginList.vue';
import { RconError, cssPluginsCommand, rcon } from '../api/gameap';
import { apiErrorMessage, deletePlugin, getState, setAttributes, togglePlugin } from '../api/plugin';
import {
    cssVersionFromMetaList,
    matchRuntimeToFolders,
    parseCssPlugins,
    parseMetaList,
    parseMetaVersion,
} from '../lib/rcon-parse';
import { prettyName } from '../lib/naming';
import { computeRowStatus } from '../lib/status';
import type {
    PlatformVersion,
    PluginRow,
    RuntimePluginInfo,
    ServerTabProps,
    StateResponse,
} from '../types';

const props = defineProps<ServerTabProps>();
const { trans } = providePluginTrans(props.pluginId);

const state = ref<StateResponse | null>(null);
const loading = ref(false);
const loadError = ref<string | null>(null);
const mutating = ref(false);

type RconAvailability =
    | 'unknown'
    | 'ok'
    | 'offline'
    | 'no-rcon'
    | 'bad-password'
    | 'empty'
    | 'error';
const rconAvailability = ref<RconAvailability>('unknown');
const metaVersion = ref<PlatformVersion | null>(null);
const cssVersion = ref<PlatformVersion | null>(null);
const cssRuntime = ref<RuntimePluginInfo[]>([]);

const installOpen = ref(false);
const configOpen = ref(false);
const configRow = ref<PluginRow | null>(null);

const serverGame = computed(() => {
    return (
        props.server as unknown as
            | { game?: { engine?: string; engine_version?: string } }
            | undefined
    )?.game;
});
const isSource2 = computed(() => {
    const engine = serverGame.value?.engine;
    // While the server object is still loading, trust the tab-level checkGame gate.
    if (!engine) {
        return true;
    }
    if (engine.toLowerCase() !== 'source') {
        return false;
    }
    const version = serverGame.value?.engine_version;
    return !version || version.trim().startsWith('2');
});

const serverOnline = computed(() => Boolean(props.server?.process_active));

const nothingInstalled = computed(() => {
    if (!state.value) {
        return false;
    }
    const metamodPresent = state.value.metamod.installed || state.value.metamod.dir_present;
    return !metamodPresent && !state.value.css.installed;
});

const rconHint = computed(() => {
    switch (rconAvailability.value) {
        case 'offline':
            return trans('rcon_unavailable_offline');
        case 'no-rcon':
            return trans('rcon_unavailable_norcon');
        case 'bad-password':
            return trans('rcon_unavailable_badpass');
        case 'empty':
            return trans('rcon_unavailable_empty');
        case 'error':
            return trans('rcon_unavailable_error');
        default:
            return null;
    }
});

const rconOk = computed(() => rconAvailability.value === 'ok');

const rows = computed<PluginRow[]>(() => {
    if (!state.value) {
        return [];
    }
    const entries = state.value.css.plugins;
    const runtimes = matchRuntimeToFolders(
        entries.map((entry) => entry.name),
        cssRuntime.value,
    );
    return entries.map((entry, index) => {
        const runtime = runtimes[index];
        const { status, detail } = computeRowStatus({
            enabled: entry.enabled,
            missing: entry.missing,
            runtime,
            rconOk: rconOk.value,
        });
        return {
            // Enabled state is part of the key: a folder present in BOTH plugins/
            // and plugins/disabled/ (a broken state) must show as two rows, not
            // have one silently swallow the other in the table's keyed diff.
            key: `css:${entry.name}:${entry.enabled ? 'on' : 'off'}`,
            name: entry.name,
            displayName: runtime?.name ?? prettyName(entry.name),
            version: runtime?.version ?? null,
            author: runtime?.author ?? null,
            enabled: entry.enabled,
            comment: entry.comment,
            missing: entry.missing,
            runtime,
            hasConfig: entry.has_config,
            configPath: entry.config_path,
            status,
            statusDetail: detail,
            groupIndex: entry.group_index,
            groupTitle: entry.group_title,
        };
    });
});

async function refreshState(): Promise<void> {
    loading.value = true;
    loadError.value = null;
    try {
        state.value = await getState(props.pluginId, props.serverId);
    } catch (error) {
        loadError.value = apiErrorMessage(error, trans('load_failed'));
    } finally {
        loading.value = false;
    }
}

async function refreshRcon(): Promise<void> {
    if (!serverOnline.value) {
        rconAvailability.value = 'offline';
        metaVersion.value = null;
        cssVersion.value = null;
        cssRuntime.value = [];
        return;
    }
    try {
        const metaVersionOut = await rcon(props.serverId, 'meta version');
        const metaListOut = await rcon(props.serverId, 'meta list');
        const cssPluginsOut = await rcon(props.serverId, 'css_plugins list');
        metaVersion.value = parseMetaVersion(metaVersionOut);
        cssVersion.value = cssVersionFromMetaList(parseMetaList(metaListOut));
        cssRuntime.value = parseCssPlugins(cssPluginsOut);
        rconAvailability.value = 'ok';
    } catch (error) {
        applyRconFailure(error);
    }
}

/** Mark the console unavailable after a failed RCON call and drop runtime data. */
function applyRconFailure(error: unknown): void {
    rconAvailability.value = error instanceof RconError ? error.reason : 'error';
    metaVersion.value = null;
    cssVersion.value = null;
    cssRuntime.value = [];
}

async function refreshAll(): Promise<void> {
    await Promise.all([refreshState(), refreshRcon()]);
}

function toast(type: 'success' | 'error' | 'info', text: string): void {
    window.$message?.[type]?.(text);
}

async function onToggle(row: PluginRow, value: boolean): Promise<void> {
    mutating.value = true;
    try {
        await togglePlugin(props.pluginId, props.serverId, row.name, value);
        toast('success', trans(value ? 'toggled_on' : 'toggled_off', { name: row.displayName }));
        await refreshState();
    } catch (error) {
        toast('error', apiErrorMessage(error, trans('op_failed')));
    } finally {
        mutating.value = false;
    }
}

/** Hot load/unload over RCON — the CS2 analogue of amxx pause/unpause. */
async function onHotAction(row: PluginRow, action: 'load' | 'unload'): Promise<void> {
    mutating.value = true;
    try {
        const output =
            action === 'unload'
                ? await cssPluginsCommand(props.serverId, 'stop', row.runtime?.name ?? row.name)
                : await cssPluginsCommand(
                      props.serverId,
                      'load',
                      `${row.name}/${row.name}.dll`,
                  );
        cssRuntime.value = parseCssPlugins(
            await rcon(props.serverId, 'css_plugins list'),
        );
        rconAvailability.value = 'ok';
        const runtime = rows.value.find((item) => item.name === row.name)?.runtime ?? null;
        const succeeded =
            action === 'unload' ? runtime?.status !== 'running' : runtime?.status === 'running';
        if (succeeded) {
            toast('success', trans(action === 'unload' ? 'unloaded_ok' : 'loaded_ok', { name: row.displayName }));
        } else {
            toast(
                'error',
                output ||
                    trans(action === 'unload' ? 'unload_failed' : 'load_failed_named', {
                        name: row.displayName,
                    }),
            );
        }
    } catch (error) {
        applyRconFailure(error);
        toast(
            'error',
            error instanceof RconError && error.reason === 'bad-password'
                ? trans('rcon_unavailable_badpass')
                : apiErrorMessage(error, trans('op_failed')),
        );
    } finally {
        mutating.value = false;
    }
}

async function onSetComment(row: PluginRow, text: string): Promise<void> {
    const comment = text.trim() || null;
    // A comment is cosmetic — skip a no-op write.
    if (comment === (row.comment ?? null)) {
        return;
    }
    mutating.value = true;
    try {
        await setAttributes(props.pluginId, props.serverId, row.name, comment, row.groupTitle);
        toast('success', trans('comment_saved', { name: row.displayName }));
        await refreshState();
    } catch (error) {
        toast('error', apiErrorMessage(error, trans('op_failed')));
    } finally {
        mutating.value = false;
    }
}

function onDelete(row: PluginRow): void {
    window.$dialog?.success({
        title: trans('delete_title', { name: row.displayName }),
        content: trans('delete_text'),
        positiveText: trans('yes'),
        negativeText: trans('no'),
        closable: false,
        onPositiveClick: async () => {
            mutating.value = true;
            try {
                await deletePlugin(props.pluginId, props.serverId, row.name);
                toast('success', trans('deleted', { name: row.displayName }));
                await refreshState();
            } catch (error) {
                toast('error', apiErrorMessage(error, trans('op_failed')));
            } finally {
                mutating.value = false;
            }
        },
    });
}

async function applyBulkToggle(bulkRows: PluginRow[], value: boolean): Promise<void> {
    mutating.value = true;
    let changed = 0;
    try {
        for (const row of bulkRows) {
            if (row.enabled === value || row.missing) {
                continue;
            }
            await togglePlugin(props.pluginId, props.serverId, row.name, value);
            changed += 1;
        }
        if (changed > 0) {
            toast('success', trans(value ? 'bulk_enabled' : 'bulk_disabled', { count: changed }));
            await refreshState();
        }
    } catch (error) {
        toast('error', apiErrorMessage(error, trans('op_failed')));
        await refreshState();
    } finally {
        mutating.value = false;
    }
}

function onBulk(action: 'enable' | 'disable' | 'delete', bulkRows: PluginRow[]): void {
    if (action !== 'delete') {
        void applyBulkToggle(bulkRows, action === 'enable');
        return;
    }
    if (bulkRows.length === 0) {
        return;
    }
    window.$dialog?.success({
        title: trans('bulk_delete_title', { count: bulkRows.length }),
        content: trans('bulk_delete_text'),
        positiveText: trans('yes'),
        negativeText: trans('no'),
        closable: false,
        onPositiveClick: async () => {
            mutating.value = true;
            let deleted = 0;
            try {
                for (const row of bulkRows) {
                    await deletePlugin(props.pluginId, props.serverId, row.name);
                    deleted += 1;
                }
                toast('success', trans('bulk_deleted', { count: deleted }));
            } catch (error) {
                toast('error', apiErrorMessage(error, trans('op_failed')));
            } finally {
                await refreshState();
                mutating.value = false;
            }
        },
    });
}

async function onInstalled(): Promise<void> {
    await refreshState();
}

function openConfig(row: PluginRow): void {
    configRow.value = row;
    configOpen.value = true;
}

function openFileManager(): void {
    window.location.hash = '#files';
}

// The server object can arrive after the tab mounts (async store load) —
// re-query the console once the server turns out to be online.
watch(serverOnline, (online, wasOnline) => {
    if (online && !wasOnline) {
        void refreshRcon();
    }
});

onMounted(() => {
    if (isSource2.value) {
        void refreshAll();
    }
});
</script>

<style scoped>
/* On md+ screens the platform cards sit flush above this panel and the CSS
 * card reads as a tab merged into it (see PlatformCard.vue). */
@media (min-width: 768px) {
    .platform-cards {
        margin-bottom: 0;
    }

    .plugins-panel.plugins-panel {
        border-top: none;
        border-top-left-radius: 0;
        border-top-right-radius: 0;
    }
}
</style>
