<template>
    <GModal
        :show="show"
        :title="trans('audit_title')"
        :style="{ width: '640px' }"
        transform-origin="center"
        @update:show="(value: boolean) => $emit('update:show', value)"
    >
        <Loading v-if="loading" />
        <n-empty v-else-if="entries.length === 0" :description="trans('audit_empty')" size="small" class="py-8" />
        <div v-else class="flex flex-col gap-1">
            <div
                v-for="(entry, index) in entries"
                :key="index"
                class="flex items-center gap-3 px-2 py-1.5 text-sm border-b border-stone-100 dark:border-stone-800 last:border-b-0"
            >
                <span class="text-xs text-stone-400 dark:text-stone-500 font-mono w-36 flex-shrink-0">
                    {{ formatDate(entry.ts) }}
                </span>
                <span class="badge-stone !me-0 text-[10px] flex-shrink-0">{{ entry.action }}</span>
                <span class="text-stone-800 dark:text-stone-100 font-medium truncate">
                    {{ entry.subject }}
                </span>
                <span class="ml-auto text-xs text-stone-500 dark:text-stone-400 flex-shrink-0">
                    {{ entry.user }}
                </span>
            </div>
        </div>
    </GModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { NEmpty } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

import { apiErrorMessage, getAudit } from '../api/plugin';
import type { AuditEntry } from '../types';

const props = defineProps<{
    show: boolean;
    serverId: number;
    pluginId: string;
}>();

const emit = defineEmits<{
    'update:show': [value: boolean];
}>();

const { trans } = usePluginTrans();

const entries = ref<AuditEntry[]>([]);
const loading = ref(false);

watch(
    () => props.show,
    async (shown) => {
        if (!shown) {
            return;
        }
        loading.value = true;
        try {
            entries.value = await getAudit(props.pluginId, props.serverId);
        } catch (error) {
            window.$message?.error(apiErrorMessage(error, trans('load_failed')));
            emit('update:show', false);
        } finally {
            loading.value = false;
        }
    },
);

function formatDate(unixSeconds: number): string {
    return new Date(unixSeconds * 1000).toLocaleString();
}
</script>
