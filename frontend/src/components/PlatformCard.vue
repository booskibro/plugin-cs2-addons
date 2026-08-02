<template>
    <n-card
        size="small"
        class="platform-card"
        :class="active ? 'platform-card--active' : 'platform-card--inactive'"
    >
        <div class="flex items-start gap-3">
            <div
                class="w-10 h-10 rounded-lg bg-stone-700 dark:bg-stone-900 text-white flex items-center justify-center flex-shrink-0"
            >
                <i :class="isMetamod ? 'fa-solid fa-plug fa-lg' : 'fa-solid fa-puzzle-piece fa-lg'"></i>
            </div>
            <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2 flex-wrap">
                    <span class="font-semibold text-stone-800 dark:text-stone-100">{{ title }}</span>
                    <n-tooltip v-if="notActive" trigger="hover">
                        <template #trigger>
                            <GStatusBadge status="warning" :text="trans('status_not_active')" />
                        </template>
                        {{ trans('not_active_hint') }}
                    </n-tooltip>
                    <GStatusBadge
                        v-else-if="!installed"
                        status="error"
                        :text="trans('status_not_installed')"
                    />
                </div>
                <div
                    v-if="installed || notActive"
                    class="text-xs text-stone-500 dark:text-stone-400 font-mono mt-0.5 truncate"
                >
                    <template v-if="version">v{{ version.version }} · </template>
                    <template v-else>{{ trans('version_unknown') }} · </template>{{ dirPath }}
                </div>
                <div v-else class="text-xs text-stone-500 dark:text-stone-400 mt-0.5">
                    {{ trans(isMetamod ? 'metamod_desc' : 'css_desc') }}
                </div>
            </div>
        </div>

        <template v-if="isMetamod">
            <div v-if="!installed && !notActive" class="mt-3 text-sm text-stone-500 dark:text-stone-400">
                {{ trans('install_hint_metamod') }}
            </div>
            <div v-else class="mt-3 text-sm text-stone-600 dark:text-stone-300">
                {{ trans('metamod_desc') }}
            </div>
        </template>

        <template v-else-if="installed">
            <div class="mt-3 flex flex-wrap gap-x-6 gap-y-1 text-sm text-stone-600 dark:text-stone-300">
                <span>
                    {{ trans('stats_total') }}:
                    <span class="font-medium text-stone-800 dark:text-stone-100">{{ rows.length }}</span>
                </span>
                <span>
                    {{ trans('stats_enabled') }}:
                    <span class="font-medium text-stone-800 dark:text-stone-100">{{ enabledCount }}</span>
                </span>
                <span v-if="errorCount" class="text-red-500 dark:text-red-400">
                    {{ trans('stats_errors') }}: {{ errorCount }}
                </span>
            </div>
        </template>

        <template v-else>
            <div class="mt-3 text-sm text-stone-500 dark:text-stone-400">
                {{ trans('install_hint_css') }}
            </div>
        </template>
    </n-card>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NCard, NTooltip } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

import type { PlatformVersion, PluginRow, StateResponse } from '../types';

const props = defineProps<{
    kind: 'metamod' | 'css';
    state: StateResponse;
    version: PlatformVersion | null;
    rows: PluginRow[];
    active?: boolean;
}>();

const { trans } = usePluginTrans();

const isMetamod = computed(() => props.kind === 'metamod');

const installed = computed(() =>
    isMetamod.value ? props.state.metamod.installed : props.state.css.installed,
);

/** Metamod addons dir exists, but gameinfo.gi does not load it. */
const notActive = computed(
    () => isMetamod.value && !props.state.metamod.installed && props.state.metamod.dir_present,
);

const title = computed(() => {
    if (props.version) {
        return props.version.build;
    }
    return isMetamod.value ? 'Metamod:Source' : 'CounterStrikeSharp';
});

const dirPath = computed(() =>
    isMetamod.value ? props.state.paths.metamod_dir : props.state.paths.css_dir,
);

const enabledCount = computed(() => props.rows.filter((row) => row.enabled).length);
const errorCount = computed(
    () => props.rows.filter((row) => row.status === 'error' || row.status === 'missing').length,
);
</script>

<style scoped>
/*
 * On md+ screens the CSS card (the one carrying the plugin list) merges with
 * the panel below it: no bottom rounding, no bottom border. The Metamod card
 * keeps its bottom border, which reads as the tab-row line. Doubled class
 * selectors keep specificity above naive-ui's runtime-injected .n-card styles.
 */
@media (min-width: 768px) {
    .platform-card {
        border-bottom-left-radius: 0;
        border-bottom-right-radius: 0;
    }

    .platform-card.platform-card--active {
        border-bottom-color: transparent;
    }

    .platform-card.platform-card--inactive {
        background-color: #f5f5f4;
    }
}
</style>

<!-- Dark-theme variants. The host panel toggles .dark on <html>, and scoped
     :global() selectors get mangled by the build — keep these unscoped. -->
<style>
@media (min-width: 768px) {
    .dark .platform-card.platform-card--inactive {
        background-color: rgba(12, 10, 9, 0.6);
    }
}
</style>
