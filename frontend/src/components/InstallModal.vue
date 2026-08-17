<template>
    <GModal
        :show="show"
        :title="trans('install_title')"
        :style="{ width: '600px' }"
        transform-origin="center"
        @update:show="onUpdateShow"
    >
        <div class="space-y-3">
            <n-upload
                v-if="!file"
                :default-upload="false"
                :show-file-list="false"
                accept=".dll,.zip"
                @change="onUploadChange"
            >
                <n-upload-dragger>
                    <div class="flex flex-col items-center gap-2 py-6">
                        <GIcon name="upload" class="text-4xl text-stone-400" />
                        <p class="text-stone-700 dark:text-stone-300 font-medium">
                            {{ trans('drop_hint') }}
                        </p>
                        <p class="text-sm text-stone-500 dark:text-stone-500">
                            {{ trans('file_hint') }}
                        </p>
                    </div>
                </n-upload-dragger>
            </n-upload>

            <template v-else>
                <div
                    class="flex items-center gap-3 p-3 rounded border border-stone-200 dark:border-stone-700 bg-stone-50 dark:bg-stone-900"
                >
                    <GIcon name="file-code" size="lg" class="text-stone-400" />
                    <div class="min-w-0 flex-1">
                        <div class="font-mono text-sm text-stone-800 dark:text-stone-100 truncate">
                            {{ file.name }}
                        </div>
                        <div class="text-xs text-stone-400">{{ prettySize }}</div>
                    </div>
                    <button
                        v-if="!uploading"
                        class="text-stone-400 hover:text-stone-600 dark:hover:text-stone-200"
                        @click="file = null"
                    >
                        <GIcon name="xmark" />
                    </button>
                </div>

                <n-progress
                    v-if="uploading"
                    type="line"
                    :percentage="progress"
                    :show-indicator="false"
                    :height="8"
                    :border-radius="4"
                    processing
                />
            </template>

            <n-alert v-if="validationError" type="warning" :show-icon="true">
                {{ validationError }}
            </n-alert>

            <div
                v-if="isOverwrite && !validationError"
                class="border border-orange-300 dark:border-orange-800 rounded p-3 bg-orange-50 dark:bg-orange-950/40"
            >
                <div class="flex items-center gap-2">
                    <GIcon name="warning" class="text-orange-500" />
                    <strong class="text-orange-700 dark:text-orange-300">
                        {{ trans('overwrite_title') }}
                    </strong>
                </div>
                <p class="mt-1 text-sm text-orange-700 dark:text-orange-300">
                    {{ trans('overwrite_text') }}
                </p>
            </div>

            <div class="flex items-center justify-end flex-wrap gap-2">
                <div class="text-xs text-stone-400 dark:text-stone-500 font-mono">→ {{ targetPath }}</div>
            </div>
        </div>

        <template #footer>
            <GButton
                :color="isOverwrite ? 'orange' : 'green'"
                :disabled="!file || Boolean(validationError) || uploading"
                @click="install"
            >
                <GIcon :name="uploading ? 'spinner' : isOverwrite ? 'refresh' : 'download'" class="mr-1" />
                {{
                    uploading
                        ? trans('uploading')
                        : isOverwrite
                          ? trans('overwrite')
                          : trans('install')
                }}
            </GButton>
        </template>
    </GModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { NAlert, NProgress, NUpload, NUploadDragger } from 'naive-ui';
import type { UploadFileInfo } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

import { fmEnsureDirectory, fmUploadFile } from '../api/gameap';
import { apiErrorMessage, installArchive, registerPlugin } from '../api/plugin';
import { httpStatus } from '../lib/http-error';
import { fileExtension, fileStem, prettyName } from '../lib/naming';
import type { StatePaths } from '../types';

const props = defineProps<{
    show: boolean;
    serverId: number;
    pluginId: string;
    paths: StatePaths;
    existingNames: string[];
}>();

const emit = defineEmits<{
    'update:show': [value: boolean];
    installed: [replaced: boolean];
}>();

const { trans } = usePluginTrans();

const file = ref<File | null>(null);
const uploading = ref(false);
const progress = ref(0);

watch(
    () => props.show,
    (shown) => {
        if (shown) {
            file.value = null;
            uploading.value = false;
            progress.value = 0;
        }
    },
);

const validationError = computed(() => {
    if (!file.value) {
        return null;
    }
    const extension = fileExtension(file.value.name);
    return extension === 'dll' || extension === 'zip' ? null : trans('wrong_type');
});

const isZip = computed(() => Boolean(file.value) && fileExtension(file.value!.name) === 'zip');

/** CounterStrikeSharp loads plugins/<Name>/<Name>.dll — the folder is the dll stem. */
const folderName = computed(() => (file.value ? fileStem(file.value.name) : null));

/** The picked dll matches an already known plugin folder. Zips learn this
 * server-side (the backend answers 409 and we ask before retrying). */
const isOverwrite = computed(() => {
    if (isZip.value) {
        return false;
    }
    const name = folderName.value?.toLowerCase();
    if (!name) {
        return false;
    }
    return props.existingNames.some((existing) => existing.toLowerCase() === name);
});

const targetPath = computed(() =>
    isZip.value
        ? `${props.paths.css_plugins_dir}/`
        : `${props.paths.css_plugins_dir}/${folderName.value ?? '…'}/`,
);

const prettySize = computed(() => {
    if (!file.value) {
        return '';
    }
    const size = file.value.size;
    if (size < 1024) {
        return `${size} B`;
    }
    if (size < 1024 * 1024) {
        return `${Math.round(size / 1024)} KB`;
    }
    return `${(size / 1024 / 1024).toFixed(1)} MB`;
});

function onUploadChange(payload: { file: UploadFileInfo }): void {
    file.value = payload.file.file ?? null;
}

function onUpdateShow(value: boolean): void {
    if (!uploading.value) {
        emit('update:show', value);
    }
}

async function install(): Promise<void> {
    const picked = file.value;
    const name = folderName.value;
    if (!picked || !name || validationError.value) {
        return;
    }
    if (isZip.value) {
        await installZip(picked);
        return;
    }
    const replaced = isOverwrite.value;
    uploading.value = true;
    progress.value = 0;
    try {
        await fmEnsureDirectory(props.serverId, props.paths.css_plugins_dir, name);
        await fmUploadFile(props.serverId, `${props.paths.css_plugins_dir}/${name}`, picked, (percent) => {
            progress.value = percent;
        });
        await registerPlugin(props.pluginId, props.serverId, {
            name,
            force: replaced,
        });
        window.$message?.success(
            trans(replaced ? 'updated_toast' : 'installed_toast', { name: prettyName(name) }),
        );
        emit('installed', replaced);
        emit('update:show', false);
    } catch (error) {
        window.$message?.error(apiErrorMessage(error, trans('op_failed')));
    } finally {
        uploading.value = false;
    }
}

/** Zip flow: file-manager upload (no size squeeze), then the backend unpacks,
 * detects the layout and registers whatever plugin folders it created. */
async function installZip(picked: File): Promise<void> {
    uploading.value = true;
    progress.value = 0;
    const archivePath = `${props.paths.css_dir}/${picked.name}`;
    try {
        await fmUploadFile(props.serverId, props.paths.css_dir, picked, (percent) => {
            progress.value = percent;
        });
        await finishZipInstall(archivePath, false);
    } catch (error) {
        if (httpStatus(error) === 409) {
            confirmZipOverwrite(archivePath, apiErrorMessage(error, ''));
            return; // uploading is reset by the dialog path
        }
        window.$message?.error(apiErrorMessage(error, trans('op_failed')));
        uploading.value = false;
    }
}

function confirmZipOverwrite(archivePath: string, detail: string): void {
    window.$dialog?.warning({
        title: trans('overwrite_title'),
        content: detail || trans('overwrite_text'),
        positiveText: trans('overwrite'),
        negativeText: trans('no'),
        onPositiveClick: async () => {
            try {
                await finishZipInstall(archivePath, true);
            } catch (error) {
                window.$message?.error(apiErrorMessage(error, trans('op_failed')));
                uploading.value = false;
            }
        },
        onNegativeClick: () => {
            uploading.value = false;
        },
        onClose: () => {
            uploading.value = false;
        },
    });
}

async function finishZipInstall(archivePath: string, force: boolean): Promise<void> {
    const result = await installArchive(props.pluginId, props.serverId, archivePath, force);
    window.$message?.success(
        trans('zip_installed_toast', {
            folders: result.folders.join(', ') || trans('group_other'),
            count: result.files_written,
        }),
    );
    uploading.value = false;
    emit('installed', force);
    emit('update:show', false);
}
</script>
