<template>
    <GModal
        :show="show"
        :title="trans('snapshots_title')"
        :style="{ width: '640px' }"
        transform-origin="center"
        @update:show="(value: boolean) => $emit('update:show', value)"
    >
        <div class="flex items-center gap-2 mb-3">
            <GButton color="green" size="small" :disabled="busy" @click="create">
                <GIcon name="save" /><span class="ml-1">{{ trans('snapshot_create') }}</span>
            </GButton>
            <span class="text-xs text-stone-400 dark:text-stone-500">
                {{ trans('snapshots_retention') }}
            </span>
        </div>

        <Loading v-if="loading" />
        <n-empty v-else-if="snapshots.length === 0" :description="trans('snapshots_empty')" size="small" />
        <div v-else class="flex flex-col gap-1.5">
            <div
                v-for="snapshot in snapshots"
                :key="snapshot.name"
                class="flex items-center gap-3 px-3 py-2 rounded border border-stone-200 dark:border-stone-700 text-sm"
            >
                <GIcon name="file-lines" size="sm" class="text-stone-400" />
                <div class="min-w-0 flex-1">
                    <div class="text-stone-800 dark:text-stone-100">
                        {{ formatDate(snapshot.created_at) }}
                    </div>
                    <div class="text-xs text-stone-400 dark:text-stone-500 font-mono truncate">
                        {{ snapshot.name }} · {{ formatSize(snapshot.size) }}
                    </div>
                </div>
                <a
                    class="link !text-xs cursor-pointer"
                    :href="downloadHref(snapshot)"
                    target="_blank"
                    rel="noopener"
                >
                    {{ trans('snapshot_download') }}
                </a>
                <GButton color="white" size="small" :disabled="busy" @click="restore(snapshot)">
                    <GIcon name="refresh" /><span class="ml-1">{{ trans('snapshot_restore') }}</span>
                </GButton>
                <GButton color="red" size="small" :disabled="busy" @click="removeSnapshot(snapshot)">
                    <GIcon name="delete" />
                </GButton>
            </div>
        </div>

        <div class="mt-3 text-xs text-stone-400 dark:text-stone-500">
            {{ trans('snapshots_transfer_hint') }}
        </div>
    </GModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { NEmpty } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

import {
    apiErrorMessage,
    createSnapshot,
    deleteSnapshot,
    listSnapshots,
    restoreSnapshot,
} from '../api/plugin';
import type { SnapshotInfo } from '../types';

const props = defineProps<{
    show: boolean;
    serverId: number;
    pluginId: string;
}>();

const emit = defineEmits<{
    'update:show': [value: boolean];
    restored: [];
}>();

const { trans } = usePluginTrans();

const snapshots = ref<SnapshotInfo[]>([]);
const loading = ref(false);
const busy = ref(false);

watch(
    () => props.show,
    (shown) => {
        if (shown) {
            void refresh();
        }
    },
);

async function refresh(): Promise<void> {
    loading.value = true;
    try {
        snapshots.value = await listSnapshots(props.pluginId, props.serverId);
    } catch (error) {
        window.$message?.error(apiErrorMessage(error, trans('load_failed')));
    } finally {
        loading.value = false;
    }
}

async function create(): Promise<void> {
    busy.value = true;
    try {
        await createSnapshot(props.pluginId, props.serverId);
        window.$message?.success(trans('snapshot_created'));
        await refresh();
    } catch (error) {
        window.$message?.error(apiErrorMessage(error, trans('op_failed')));
    } finally {
        busy.value = false;
    }
}

function restore(snapshot: SnapshotInfo): void {
    window.$dialog?.warning({
        title: trans('snapshot_restore_title'),
        content: trans('snapshot_restore_text', { date: formatDate(snapshot.created_at) }),
        positiveText: trans('yes'),
        negativeText: trans('no'),
        onPositiveClick: async () => {
            busy.value = true;
            try {
                await restoreSnapshot(props.pluginId, props.serverId, snapshot.name);
                window.$message?.success(trans('snapshot_restored'));
                emit('restored');
            } catch (error) {
                window.$message?.error(apiErrorMessage(error, trans('op_failed')));
            } finally {
                busy.value = false;
            }
        },
    });
}

async function removeSnapshot(snapshot: SnapshotInfo): Promise<void> {
    busy.value = true;
    try {
        await deleteSnapshot(props.pluginId, props.serverId, snapshot.name);
        await refresh();
    } catch (error) {
        window.$message?.error(apiErrorMessage(error, trans('op_failed')));
    } finally {
        busy.value = false;
    }
}

function downloadHref(snapshot: SnapshotInfo): string {
    const params = new URLSearchParams({ disk: 'server', path: snapshot.path });
    return `/api/file-manager/${props.serverId}/download?${params.toString()}`;
}

function formatDate(unixSeconds: number): string {
    return new Date(unixSeconds * 1000).toLocaleString();
}

function formatSize(bytes: number): string {
    if (bytes >= 1024 * 1024) {
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }
    if (bytes >= 1024) {
        return `${(bytes / 1024).toFixed(0)} KB`;
    }
    return `${bytes} B`;
}
</script>
