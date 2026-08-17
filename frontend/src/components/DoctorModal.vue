<template>
    <GModal
        :show="show"
        :title="trans('doctor_title')"
        :style="{ width: '640px' }"
        transform-origin="center"
        @update:show="(value: boolean) => $emit('update:show', value)"
    >
        <Loading v-if="loading" />
        <template v-else>
            <div class="flex flex-col gap-1.5">
                <div
                    v-for="check in allChecks"
                    :key="check.id"
                    class="flex items-start gap-3 px-3 py-2 rounded border border-stone-200 dark:border-stone-700"
                >
                    <span
                        class="mt-0.5 inline-flex items-center justify-center w-5 h-5 rounded-full flex-shrink-0"
                        :class="badgeClass(check.status)"
                    >
                        <i :class="iconClass(check.status)" class="text-[10px]"></i>
                    </span>
                    <div class="min-w-0">
                        <div class="text-sm font-medium text-stone-800 dark:text-stone-100">
                            {{ trans(`doctor_check_${check.id}`) }}
                        </div>
                        <div class="text-xs text-stone-500 dark:text-stone-400 break-words">
                            {{ check.detail }}
                        </div>
                    </div>
                </div>
            </div>
            <div class="mt-3 flex items-center gap-2">
                <GButton color="white" size="small" :disabled="loading" @click="refresh">
                    <GIcon name="refresh" /><span class="ml-1">{{ trans('doctor_recheck') }}</span>
                </GButton>
                <span v-if="summary" class="text-xs" :class="summaryClass">{{ summary }}</span>
            </div>
        </template>
    </GModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { usePluginTrans } from '@gameap/plugin-sdk';

import { apiErrorMessage, getDoctor } from '../api/plugin';
import type { DoctorCheck } from '../types';

const props = defineProps<{
    show: boolean;
    serverId: number;
    pluginId: string;
    /** Checks only the tab itself can make (RCON state, launch command). */
    frontendChecks: DoctorCheck[];
}>();

const emit = defineEmits<{
    'update:show': [value: boolean];
}>();

const { trans } = usePluginTrans();

const serverChecks = ref<DoctorCheck[]>([]);
const loading = ref(false);

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
        serverChecks.value = await getDoctor(props.pluginId, props.serverId);
    } catch (error) {
        window.$message?.error(apiErrorMessage(error, trans('load_failed')));
        emit('update:show', false);
    } finally {
        loading.value = false;
    }
}

const allChecks = computed<DoctorCheck[]>(() => [...props.frontendChecks, ...serverChecks.value]);

const summary = computed(() => {
    const fails = allChecks.value.filter((check) => check.status === 'fail').length;
    const warns = allChecks.value.filter((check) => check.status === 'warn').length;
    if (fails === 0 && warns === 0) {
        return trans('doctor_all_ok');
    }
    return trans('doctor_summary', { fails, warns });
});

const summaryClass = computed(() =>
    allChecks.value.some((check) => check.status === 'fail')
        ? 'text-red-500 dark:text-red-400'
        : allChecks.value.some((check) => check.status === 'warn')
          ? 'text-orange-500 dark:text-orange-400'
          : 'text-emerald-600 dark:text-emerald-400',
);

function badgeClass(status: DoctorCheck['status']): string {
    switch (status) {
        case 'ok':
            return 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300';
        case 'warn':
            return 'bg-orange-100 text-orange-700 dark:bg-orange-900 dark:text-orange-300';
        default:
            return 'bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300';
    }
}

function iconClass(status: DoctorCheck['status']): string {
    switch (status) {
        case 'ok':
            return 'fa-solid fa-check';
        case 'warn':
            return 'fa-solid fa-exclamation';
        default:
            return 'fa-solid fa-xmark';
    }
}
</script>
