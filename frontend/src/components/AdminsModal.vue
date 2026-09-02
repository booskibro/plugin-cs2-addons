<template>
    <GModal
        :show="show"
        :title="trans('admins_title')"
        :style="{ width: '860px' }"
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
                    <n-data-table
                        :columns="adminColumns"
                        :data="admins"
                        :row-key="rowKey"
                        :bordered="false"
                        :single-line="true"
                        size="small"
                        :max-height="380"
                        :scroll-x="700"
                    >
                        <template #empty>
                            <n-empty :description="trans('admins_empty')" size="small" class="py-4" />
                        </template>
                    </n-data-table>
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
import { computed, h, ref, watch } from 'vue';
import {
    NDataTable,
    NEmpty,
    NInput,
    NInputNumber,
    NTabPane,
    NTabs,
    type DataTableColumns,
} from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

import { fmDownloadText, fmUploadFile } from '../api/gameap';
import { apiErrorMessage } from '../api/plugin';

interface AdminDraft {
    /** Stable table row key: the name is edited, so it cannot be the key. */
    key: number;
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

let nextKey = 0;

const adminsPath = computed(() => `${props.cssDir}/configs/admins.json`);
const groupsPath = computed(() => `${props.cssDir}/configs/admin_groups.json`);

const rowKey = (row: AdminDraft): number => row.key;

// Editable cells: naive-ui renders them, so the drafts stay plain objects and
// the save path is unchanged.
const adminColumns = computed<DataTableColumns<AdminDraft>>(() => [
    {
        title: trans('admins_col_name'),
        key: 'name',
        minWidth: 130,
        render: (row) =>
            h(NInput, {
                value: row.name,
                size: 'small',
                placeholder: 'Nick',
                'onUpdate:value': (value: string) => {
                    row.name = value;
                },
            }),
    },
    {
        title: trans('admins_col_identity'),
        key: 'identity',
        minWidth: 170,
        render: (row) =>
            h(NInput, {
                value: row.identity,
                size: 'small',
                class: 'font-mono',
                placeholder: '76561198...',
                'onUpdate:value': (value: string) => {
                    row.identity = value;
                },
            }),
    },
    {
        title: trans('admins_col_flags'),
        key: 'flags',
        minWidth: 210,
        render: (row) =>
            h(NInput, {
                value: row.flags,
                size: 'small',
                class: 'font-mono',
                placeholder: '@css/generic, @css/ban',
                'onUpdate:value': (value: string) => {
                    row.flags = value;
                },
            }),
    },
    {
        title: trans('admins_col_immunity'),
        key: 'immunity',
        width: 110,
        render: (row) =>
            h(NInputNumber, {
                value: row.immunity,
                size: 'small',
                min: 0,
                max: 100,
                showButton: false,
                'onUpdate:value': (value: number | null) => {
                    row.immunity = value ?? 0;
                },
            }),
    },
    {
        title: '',
        key: 'actions',
        width: 48,
        align: 'center',
        render: (row) =>
            h(
                'button',
                {
                    class: 'text-stone-400 hover:text-red-500',
                    title: trans('action_delete'),
                    onClick: () => removeAdmin(row.key),
                },
                h('i', { class: 'fa-solid fa-trash-can' }),
            ),
    },
]);

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
                key: nextKey++,
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
        key: nextKey++,
        name: '',
        identity: '',
        flags: '@css/generic',
        immunity: 0,
        extra: {},
    });
}

function removeAdmin(key: number): void {
    admins.value = admins.value.filter((admin) => admin.key !== key);
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
