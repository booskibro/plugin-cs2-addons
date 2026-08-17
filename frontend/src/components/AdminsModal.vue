<template>
    <GModal
        :show="show"
        :title="trans('admins_title')"
        :style="{ width: '760px' }"
        transform-origin="center"
        @update:show="(value: boolean) => $emit('update:show', value)"
    >
        <Loading v-if="loading" />
        <template v-else>
            <n-tabs v-model:value="tab" type="line" size="small">
                <n-tab-pane name="admins" :tab="trans('admins_tab_admins')">
                    <div class="mb-2 text-xs text-stone-400 dark:text-stone-500 font-mono">
                        {{ adminsPath }}
                    </div>
                    <div class="flex flex-col gap-1.5">
                        <div
                            class="hidden md:grid grid-cols-[1fr_1.4fr_1.6fr_90px_32px] gap-2 text-xs uppercase tracking-wide text-stone-400 dark:text-stone-500 px-1"
                        >
                            <span>{{ trans('admins_col_name') }}</span>
                            <span>{{ trans('admins_col_identity') }}</span>
                            <span>{{ trans('admins_col_flags') }}</span>
                            <span>{{ trans('admins_col_immunity') }}</span>
                            <span></span>
                        </div>
                        <div
                            v-for="(admin, index) in admins"
                            :key="index"
                            class="grid grid-cols-1 md:grid-cols-[1fr_1.4fr_1.6fr_90px_32px] gap-2 items-center"
                        >
                            <n-input v-model:value="admin.name" size="small" placeholder="Nick" />
                            <n-input
                                v-model:value="admin.identity"
                                size="small"
                                placeholder="76561198..."
                                class="font-mono"
                            />
                            <n-input
                                v-model:value="admin.flags"
                                size="small"
                                placeholder="@css/generic, @css/ban"
                                class="font-mono"
                            />
                            <n-input-number
                                v-model:value="admin.immunity"
                                size="small"
                                :min="0"
                                :max="100"
                                :show-button="false"
                            />
                            <button
                                class="text-stone-400 hover:text-red-500"
                                :title="trans('action_delete')"
                                @click="admins.splice(index, 1)"
                            >
                                <GIcon name="delete" size="sm" />
                            </button>
                        </div>
                    </div>
                    <GButton color="white" size="small" class="mt-2" @click="addAdmin">
                        <i class="fa-solid fa-plus"></i><span class="ml-1">{{ trans('admins_add') }}</span>
                    </GButton>
                    <div class="mt-2 text-xs text-stone-400 dark:text-stone-500">
                        {{ trans('admins_hint') }}
                    </div>
                </n-tab-pane>
                <n-tab-pane name="groups" :tab="trans('admins_tab_groups')">
                    <div class="mb-2 text-xs text-stone-400 dark:text-stone-500 font-mono">
                        {{ groupsPath }}
                    </div>
                    <n-input
                        v-model:value="groupsText"
                        type="textarea"
                        :rows="12"
                        class="font-mono"
                        placeholder="{}"
                    />
                </n-tab-pane>
            </n-tabs>
        </template>

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
import { NInput, NInputNumber, NTabPane, NTabs } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

import { fmDownloadText, fmUploadFile } from '../api/gameap';
import { apiErrorMessage } from '../api/plugin';

interface AdminDraft {
    name: string;
    identity: string;
    /** Comma-separated in the editor, array on disk. */
    flags: string;
    immunity: number;
    /** Fields this editor does not manage, preserved verbatim. */
    extra: Record<string, unknown>;
}

const props = defineProps<{
    show: boolean;
    serverId: number;
    /** Server-dir-relative CounterStrikeSharp dir, e.g. game/csgo/addons/counterstrikesharp. */
    cssDir: string;
}>();

const emit = defineEmits<{
    'update:show': [value: boolean];
}>();

const { trans } = usePluginTrans();

const tab = ref<'admins' | 'groups'>('admins');
const admins = ref<AdminDraft[]>([]);
const groupsText = ref('');
const loading = ref(false);
const saving = ref(false);

const adminsPath = computed(() => `${props.cssDir}/configs/admins.json`);
const groupsPath = computed(() => `${props.cssDir}/configs/admin_groups.json`);

watch(
    () => props.show,
    async (shown) => {
        if (!shown) {
            return;
        }
        loading.value = true;
        try {
            admins.value = parseAdmins(await downloadOr(adminsPath.value, '{}'));
            groupsText.value = await downloadOr(groupsPath.value, '{}');
        } finally {
            loading.value = false;
        }
    },
);

async function downloadOr(path: string, fallback: string): Promise<string> {
    try {
        const text = await fmDownloadText(props.serverId, path);
        return text.trim() === '' ? fallback : text;
    } catch {
        return fallback; // missing file = empty config
    }
}

function parseAdmins(text: string): AdminDraft[] {
    let root: unknown;
    try {
        root = JSON.parse(text);
    } catch {
        window.$message?.error(trans('admins_parse_failed'));
        return [];
    }
    if (typeof root !== 'object' || root === null || Array.isArray(root)) {
        return [];
    }
    return Object.entries(root as Record<string, Record<string, unknown>>).map(
        ([name, entry]) => {
            const { identity, flags, immunity, ...extra } = entry ?? {};
            return {
                name,
                identity: typeof identity === 'string' ? identity : '',
                flags: Array.isArray(flags) ? flags.join(', ') : '',
                immunity: typeof immunity === 'number' ? immunity : 0,
                extra,
            };
        },
    );
}

function serializeAdmins(): string {
    const root: Record<string, unknown> = {};
    for (const admin of admins.value) {
        const name = admin.name.trim();
        if (!name) {
            continue;
        }
        root[name] = {
            ...admin.extra,
            identity: admin.identity.trim(),
            flags: admin.flags
                .split(',')
                .map((flag) => flag.trim())
                .filter((flag) => flag.length > 0),
            immunity: admin.immunity,
        };
    }
    return `${JSON.stringify(root, null, 2)}\n`;
}

function addAdmin(): void {
    admins.value.push({
        name: '',
        identity: '',
        flags: '@css/generic',
        immunity: 0,
        extra: {},
    });
}

async function save(): Promise<void> {
    let groups: string;
    try {
        groups = `${JSON.stringify(JSON.parse(groupsText.value || '{}'), null, 2)}\n`;
    } catch (error) {
        window.$message?.error(trans('admins_groups_invalid', { error: String(error) }));
        return;
    }
    saving.value = true;
    try {
        await uploadTo(adminsPath.value, serializeAdmins());
        await uploadTo(groupsPath.value, groups);
        window.$message?.success(trans('admins_saved'));
        emit('update:show', false);
    } catch (error) {
        window.$message?.error(apiErrorMessage(error, trans('op_failed')));
    } finally {
        saving.value = false;
    }
}

async function uploadTo(path: string, content: string): Promise<void> {
    const idx = path.lastIndexOf('/');
    const directory = path.slice(0, idx);
    const name = path.slice(idx + 1);
    await fmUploadFile(
        props.serverId,
        directory,
        new File([content], name, { type: 'application/json' }),
    );
}
</script>
