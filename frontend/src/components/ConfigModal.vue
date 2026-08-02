<template>
    <GModal
        :show="show"
        :title="title"
        :style="{ width: '680px' }"
        transform-origin="center"
        @update:show="(value: boolean) => $emit('update:show', value)"
    >
        <div
            v-if="row?.configPath"
            class="mb-2 text-xs text-stone-400 dark:text-stone-500 font-mono flex items-center gap-1.5"
        >
            <GIcon name="file-lines" size="sm" /> {{ row.configPath }}
        </div>

        <Loading v-if="loading" />
        <n-input
            v-else
            v-model:value="text"
            type="textarea"
            :rows="14"
            class="font-mono"
            placeholder="#"
        />

        <template #footer>
            <GButton color="green" :disabled="loading || saving" @click="save">
                <GIcon name="save" class="mr-1" />
                {{ trans('save') }}
            </GButton>
        </template>
    </GModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { NInput } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

import { fmDownloadText, fmUploadFile } from '../api/gameap';
import { apiErrorMessage } from '../api/plugin';
import type { PluginRow } from '../types';

const props = defineProps<{
    show: boolean;
    serverId: number;
    row: PluginRow | null;
}>();

const emit = defineEmits<{
    'update:show': [value: boolean];
}>();

const { trans } = usePluginTrans();

const text = ref('');
const loading = ref(false);
const saving = ref(false);

const title = computed(() =>
    props.row ? trans('config_title', { name: props.row.name }) : trans('action_config'),
);

watch(
    () => props.show,
    async (shown) => {
        if (!shown || !props.row?.configPath) {
            return;
        }
        loading.value = true;
        text.value = '';
        try {
            text.value = await fmDownloadText(props.serverId, props.row.configPath);
        } catch (error) {
            window.$message?.error(apiErrorMessage(error, trans('config_load_failed')));
            emit('update:show', false);
        } finally {
            loading.value = false;
        }
    },
);

async function save(): Promise<void> {
    const configPath = props.row?.configPath;
    if (!configPath) {
        return;
    }
    const idx = configPath.lastIndexOf('/');
    const directory = idx > 0 ? configPath.slice(0, idx) : '.';
    const name = idx > 0 ? configPath.slice(idx + 1) : configPath;
    saving.value = true;
    try {
        await fmUploadFile(props.serverId, directory, new File([text.value], name, { type: 'text/plain' }));
        window.$message?.success(trans('config_saved'));
        emit('update:show', false);
    } catch (error) {
        window.$message?.error(apiErrorMessage(error, trans('op_failed')));
    } finally {
        saving.value = false;
    }
}
</script>
