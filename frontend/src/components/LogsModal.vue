<template>
    <GModal
        :show="show"
        :title="trans('logs_title')"
        :style="{ width: '820px' }"
        transform-origin="center"
        @update:show="(value: boolean) => $emit('update:show', value)"
    >
        <div class="flex items-center gap-2 mb-2">
            <n-input
                v-model:value="filter"
                :placeholder="trans('logs_filter_placeholder')"
                size="small"
                clearable
                class="w-72"
            />
            <GButton color="white" size="small" :disabled="loading" @click="refresh">
                <GIcon name="refresh" /><span class="ml-1">{{ trans('retry') }}</span>
            </GButton>
            <span v-if="file" class="ml-auto text-xs text-stone-400 dark:text-stone-500 font-mono truncate">
                {{ file }}
            </span>
        </div>

        <Loading v-if="loading" />
        <n-empty
            v-else-if="visibleLines.length === 0"
            :description="trans('logs_empty')"
            size="small"
            class="py-8"
        />
        <div
            v-else
            class="csa-log-box font-mono text-xs leading-5 rounded border border-stone-200 dark:border-stone-700 bg-stone-50 dark:bg-stone-900 p-2 overflow-auto"
            style="max-height: 420px"
        >
            <div
                v-for="(line, index) in visibleLines"
                :key="index"
                :class="isErrorLine(line) ? 'text-red-600 dark:text-red-400' : 'text-stone-700 dark:text-stone-300'"
                class="whitespace-pre-wrap break-all"
            >
                {{ line }}
            </div>
        </div>
    </GModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { NEmpty, NInput } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

import { apiErrorMessage, getLogs } from '../api/plugin';

const props = defineProps<{
    show: boolean;
    serverId: number;
    pluginId: string;
    /** Prefilled filter (a plugin name, when opened from an error row). */
    initialFilter: string;
}>();

const emit = defineEmits<{
    'update:show': [value: boolean];
}>();

const { trans } = usePluginTrans();

const lines = ref<string[]>([]);
const file = ref<string | null>(null);
const filter = ref('');
const loading = ref(false);

watch(
    () => props.show,
    (shown) => {
        if (shown) {
            filter.value = props.initialFilter;
            void refresh();
        }
    },
);

async function refresh(): Promise<void> {
    loading.value = true;
    try {
        const response = await getLogs(props.pluginId, props.serverId);
        lines.value = response.lines;
        file.value = response.file;
    } catch (error) {
        window.$message?.error(apiErrorMessage(error, trans('load_failed')));
    } finally {
        loading.value = false;
    }
}

const visibleLines = computed(() => {
    const query = filter.value.trim().toLowerCase();
    if (!query) {
        return lines.value;
    }
    return lines.value.filter((line) => line.toLowerCase().includes(query));
});

function isErrorLine(line: string): boolean {
    const lower = line.toLowerCase();
    return (
        lower.includes('error') ||
        lower.includes('exception') ||
        lower.includes('fail') ||
        lower.includes('warn')
    );
}
</script>
