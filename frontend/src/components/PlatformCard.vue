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
                    <n-tooltip v-if="updateAvailable" trigger="hover">
                        <template #trigger>
                            <span class="badge-orange !me-0 text-[10px]">
                                {{ trans('update_available', { version: updateVersion ?? '' }) }}
                            </span>
                        </template>
                        {{ trans('update_available_hint') }}
                    </n-tooltip>
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

            <!-- Binary Metamod plugins (.vdf aliases) -->
            <div v-if="metamodPlugins.length" class="mt-3 flex flex-col gap-1">
                <div class="text-xs uppercase tracking-wide text-stone-400 dark:text-stone-500">
                    {{ trans('vdf_plugins') }}
                </div>
                <div
                    v-for="plugin in metamodPlugins"
                    :key="`${plugin.name}:${plugin.enabled}`"
                    class="flex items-center gap-2 text-sm"
                >
                    <n-switch
                        size="small"
                        :value="plugin.enabled"
                        :disabled="busy"
                        @update:value="(value: boolean) => $emit('toggle-vdf', plugin.name, value)"
                    />
                    <span class="font-mono text-xs text-stone-700 dark:text-stone-200">
                        {{ plugin.name }}
                    </span>
                </div>
                <div class="text-[11px] text-stone-400 dark:text-stone-500">
                    {{ trans('vdf_hint') }}
                </div>
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

        <!-- platform actions -->
        <div v-if="showActions" class="mt-3 flex flex-wrap gap-2">
            <GButton
                v-if="isMetamod && notActive"
                color="green"
                size="small"
                :disabled="busy"
                @click="$emit('repair')"
            >
                <i class="fa-solid fa-wrench"></i><span class="ml-1">{{ trans('repair_gameinfo') }}</span>
            </GButton>
            <GButton
                v-if="!installed && !notActive"
                color="green"
                size="small"
                :disabled="busy"
                @click="$emit('install')"
            >
                <i class="fa-solid fa-download"></i>
                <span class="ml-1">{{ trans(busy ? 'platform_installing' : 'platform_install') }}</span>
            </GButton>
            <GButton
                v-else-if="updateAvailable"
                color="white"
                size="small"
                :disabled="busy"
                @click="$emit('install')"
            >
                <i class="fa-solid fa-download"></i>
                <span class="ml-1">
                    {{ trans(busy ? 'platform_installing' : 'platform_update', { version: updateVersion ?? '' }) }}
                </span>
            </GButton>
        </div>
    </n-card>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NCard, NSwitch, NTooltip } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

import { versionsMatch } from '../lib/version';
import type { MetamodPluginEntry, PlatformVersion, PluginRow, StateResponse } from '../types';

const props = defineProps<{
    kind: 'metamod' | 'css';
    state: StateResponse;
    version: PlatformVersion | null;
    rows: PluginRow[];
    active?: boolean;
    /** Latest upstream version, when known. */
    updateVersion?: string | null;
    metamodPlugins?: MetamodPluginEntry[];
    busy?: boolean;
}>();

defineEmits<{
    install: [];
    repair: [];
    'toggle-vdf': [name: string, enabled: boolean];
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

const metamodPlugins = computed<MetamodPluginEntry[]>(() =>
    isMetamod.value ? (props.metamodPlugins ?? props.state.metamod.plugins ?? []) : [],
);

/** Known runtime version differs from the known latest → update offer. */
const updateAvailable = computed(() => {
    if (!props.updateVersion || !props.version || !installed.value) {
        return false;
    }
    return !versionsMatch(props.version.version, props.updateVersion);
});

const showActions = computed(
    () => notActive.value || !installed.value || updateAvailable.value,
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
