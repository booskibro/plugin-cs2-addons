<template>
    <div>
        <!-- CounterStrikeSharp is not installed -->
        <div v-if="!installed" class="py-10">
            <n-empty :description="trans('css_missing')">
                <template #icon>
                    <i class="fa-solid fa-puzzle-piece fa-2x text-stone-300 dark:text-stone-600"></i>
                </template>
                <template #extra>
                    <div class="text-sm text-stone-500 dark:text-stone-400 mb-3 max-w-md mx-auto text-center">
                        {{ trans('platform_missing_hint') }}
                    </div>
                </template>
            </n-empty>
        </div>

        <template v-else>
            <!-- toolbar -->
            <div class="flex flex-wrap items-center gap-2 mb-3">
                <GButton color="green" size="small" @click="$emit('install')">
                    <GIcon name="upload" /><span class="ml-1">{{ trans('upload_file') }}</span>
                </GButton>
                <div class="flex items-center gap-2 ml-auto">
                    <n-input
                        v-model:value="search"
                        :placeholder="trans('search_placeholder')"
                        size="small"
                        clearable
                        class="w-72 sm:w-80"
                    >
                        <template #prefix>
                            <GIcon name="search" size="sm" class="text-stone-400" />
                        </template>
                    </n-input>
                    <n-select v-model:value="filter" :options="filterOptions" size="small" class="w-40" />
                </div>
            </div>

            <!-- bulk actions -->
            <div
                v-if="checked.length"
                class="flex flex-wrap items-center gap-2 mb-3 px-3 py-2 rounded bg-stone-100 dark:bg-stone-700 text-sm text-stone-700 dark:text-stone-200"
            >
                <span class="font-medium">{{ trans('selected', { count: checked.length }) }}</span>
                <GButton color="green" size="small" :disabled="busy" @click="bulk('enable')">
                    <GIcon name="play" /><span class="ml-1">{{ trans('bulk_enable') }}</span>
                </GButton>
                <GButton color="white" size="small" :disabled="busy" @click="bulk('disable')">
                    <GIcon name="stop" /><span class="ml-1">{{ trans('bulk_disable') }}</span>
                </GButton>
                <GButton color="red" size="small" :disabled="busy" @click="bulk('delete')">
                    <GIcon name="delete" /><span class="ml-1">{{ trans('bulk_delete') }}</span>
                </GButton>
                <button
                    class="ml-auto text-stone-400 hover:text-stone-600 dark:hover:text-stone-200"
                    @click="checked = []"
                >
                    <GIcon name="xmark" />
                </button>
            </div>

            <!-- table -->
            <n-data-table
                :columns="columns"
                :data="grouped"
                :bordered="false"
                :single-line="true"
                :row-key="rowKey"
                :row-class-name="rowClassName"
                :scroll-x="840"
            >
                <template #empty>
                    <n-empty :description="emptyText">
                        <template #extra>
                            <GButton
                                v-if="!search && filter === 'all'"
                                color="green"
                                size="small"
                                @click="$emit('install')"
                            >
                                <GIcon name="upload" />
                                <span class="ml-1">{{ trans('install_first') }}</span>
                            </GButton>
                        </template>
                    </n-empty>
                </template>
            </n-data-table>

            <!-- plugins dir path -->
            <div class="mt-2 flex items-center gap-2 text-xs text-stone-400 dark:text-stone-500">
                <GIcon name="file-lines" size="sm" />
                <span class="font-mono">{{ pluginsPath }}</span>
                <a class="link !text-xs cursor-pointer inline-flex items-center gap-1" @click="$emit('open-files')">
                    {{ trans('open_in_filemanager') }} <GIcon name="external-link" size="sm" />
                </a>
            </div>
        </template>
    </div>
</template>

<script setup lang="ts">
import { computed, h, ref } from 'vue';
import {
    NCheckbox,
    NDataTable,
    NEmpty,
    NInput,
    NSelect,
    NSwitch,
    NTooltip,
    type DataTableColumns,
} from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

import { hotActionForStatus } from '../lib/status';
import type { PluginRow, RowStatus } from '../types';

const props = defineProps<{
    rows: PluginRow[];
    installed: boolean;
    pluginsPath: string;
    busy: boolean;
}>();

const emit = defineEmits<{
    toggle: [row: PluginRow, value: boolean];
    'hot-action': [row: PluginRow, action: 'load' | 'unload'];
    'set-comment': [row: PluginRow, text: string];
    remove: [row: PluginRow];
    bulk: [action: 'enable' | 'disable' | 'delete', rows: PluginRow[]];
    install: [];
    configure: [row: PluginRow];
    'open-files': [];
}>();

const { trans } = usePluginTrans();

const search = ref('');
const filter = ref<'all' | 'on' | 'off' | 'err'>('all');
const checked = ref<string[]>([]);

// Key of the row whose comment is being edited inline, plus the draft text.
const editingKey = ref<string | null>(null);
const editText = ref('');

/** Group-header pseudo-row injected into the table between plugin groups. */
interface HeaderRow {
    isHeader: true;
    key: string;
    title: string;
    count: number;
}
type TableRow = HeaderRow | PluginRow;

function isHeader(row: TableRow): row is HeaderRow {
    return (row as HeaderRow).isHeader === true;
}

const filterOptions = computed(() => [
    { label: trans('filter_all'), value: 'all' },
    { label: trans('filter_on'), value: 'on' },
    { label: trans('filter_off'), value: 'off' },
    { label: trans('filter_err'), value: 'err' },
]);

const filtered = computed(() => {
    const query = search.value.trim().toLowerCase();
    return props.rows.filter((row) => {
        if (query) {
            const haystack =
                `${row.displayName} ${row.name} ${row.author ?? ''} ${row.comment ?? ''}`.toLowerCase();
            if (!haystack.includes(query)) {
                return false;
            }
        }
        switch (filter.value) {
            case 'on':
                return row.enabled;
            case 'off':
                return !row.enabled;
            case 'err':
                return row.status === 'error' || row.status === 'missing';
            default:
                return true;
        }
    });
});

// Group the filtered rows by group_index and interleave header rows. When no
// group is named (only the "Other" bucket), fall back to a flat list.
const grouped = computed<TableRow[]>(() => {
    const byIndex = new Map<number, PluginRow[]>();
    for (const row of filtered.value) {
        const list = byIndex.get(row.groupIndex);
        if (list) {
            list.push(row);
        } else {
            byIndex.set(row.groupIndex, [row]);
        }
    }
    const indices = [...byIndex.keys()].sort((a, b) => a - b);
    const showHeaders = indices.some((index) =>
        (byIndex.get(index) ?? []).some((row) => row.groupTitle !== null),
    );

    const out: TableRow[] = [];
    for (const index of indices) {
        const list = byIndex.get(index) ?? [];
        if (showHeaders) {
            out.push({
                isHeader: true,
                key: `header:${index}`,
                title: list[0]?.groupTitle ?? trans('group_other'),
                count: list.length,
            });
        }
        out.push(...list);
    }
    return out;
});

const emptyText = computed(() =>
    search.value || filter.value !== 'all' ? trans('empty_no_results') : trans('empty_no_plugins'),
);

const COLUMN_COUNT = 5;

const rowKey = (row: TableRow) => row.key;
const rowClassName = (row: TableRow) => (isHeader(row) ? 'csa-group-row' : '');

// --- selection (hand-rolled so header rows carry no checkbox) ---

const selectableKeys = computed(() => filtered.value.map((row) => row.key));
const allChecked = computed(
    () =>
        selectableKeys.value.length > 0 &&
        selectableKeys.value.every((key) => checked.value.includes(key)),
);
const someChecked = computed(
    () => !allChecked.value && selectableKeys.value.some((key) => checked.value.includes(key)),
);

function toggleChecked(key: string, value: boolean): void {
    if (value) {
        if (!checked.value.includes(key)) {
            checked.value = [...checked.value, key];
        }
    } else {
        checked.value = checked.value.filter((item) => item !== key);
    }
}

function toggleAll(value: boolean): void {
    if (value) {
        const set = new Set(checked.value);
        selectableKeys.value.forEach((key) => set.add(key));
        checked.value = [...set];
    } else {
        const drop = new Set(selectableKeys.value);
        checked.value = checked.value.filter((key) => !drop.has(key));
    }
}

// --- inline comment editing ---

function startEdit(row: PluginRow): void {
    editingKey.value = row.key;
    editText.value = row.comment ?? '';
}

function cancelEdit(): void {
    editingKey.value = null;
}

function saveComment(row: PluginRow): void {
    if (editingKey.value !== row.key) {
        return;
    }
    const text = editText.value;
    editingKey.value = null;
    emit('set-comment', row, text);
}

// --- render helpers ---

function statusMeta(row: PluginRow): { cls: string; text: string } {
    const map: Record<RowStatus, { cls: string; key: string }> = {
        running: { cls: 'badge-green', key: 'status_running' },
        enabled: { cls: 'badge-green', key: 'status_enabled' },
        stopped: { cls: 'badge-stone', key: 'status_stopped' },
        pending: { cls: 'badge-orange', key: 'status_pending' },
        error: { cls: 'badge-red', key: 'status_error' },
        missing: { cls: 'badge-red', key: 'status_missing' },
    };
    const meta = map[row.status];
    return { cls: meta.cls, text: trans(meta.key) };
}

/** Builds an inline lucide-style svg (stroke-based, 24×24 grid). */
function lucideSvg(children: ReturnType<typeof h>[], extraClass = ''): ReturnType<typeof h> {
    return h(
        'svg',
        {
            xmlns: 'http://www.w3.org/2000/svg',
            viewBox: '0 0 24 24',
            fill: 'none',
            stroke: 'currentColor',
            'stroke-width': '2',
            'stroke-linecap': 'round',
            'stroke-linejoin': 'round',
            'aria-hidden': 'true',
            class: extraClass ? `w-3.5 h-3.5 ${extraClass}` : 'w-3.5 h-3.5',
        },
        children,
    );
}

function renderStatusIcon(status: RowStatus): ReturnType<typeof h> {
    const heartPulse = (animated: boolean): ReturnType<typeof h> =>
        lucideSvg(
            [
                h('path', {
                    d: 'M2 9.5a5.5 5.5 0 0 1 9.591-3.676.56.56 0 0 0 .818 0A5.49 5.49 0 0 1 22 9.5c0 2.29-1.5 4-3 5.5l-5.492 5.313a2 2 0 0 1-3 .019L5 15c-1.5-1.5-3-3.2-3-5.5',
                }),
                h('path', { d: 'M3.22 13H9.5l.5-1 2 4.5 2-7 1.5 3.5h5.27' }),
            ],
            animated ? 'csa-heartbeat' : '',
        );
    switch (status) {
        case 'running':
            return heartPulse(true);
        case 'enabled':
            return heartPulse(false);
        case 'pending':
            return lucideSvg([
                h('circle', { cx: '12', cy: '12', r: '10' }),
                h('polyline', { points: '12 6 12 12 16 14' }),
            ]);
        case 'stopped':
            return lucideSvg([h('rect', { x: '3', y: '3', width: '18', height: '18', rx: '2' })]);
        case 'error':
            return lucideSvg([
                h('path', { d: 'm12.5 17-.5-1-.5 1h1z' }),
                h('path', {
                    d: 'M15 22a1 1 0 0 0 1-1v-1a2 2 0 0 0 1.56-3.25 8 8 0 1 0-11.12 0A2 2 0 0 0 8 20v1a1 1 0 0 0 1 1z',
                }),
                h('circle', { cx: '15', cy: '12', r: '1' }),
                h('circle', { cx: '9', cy: '12', r: '1' }),
            ]);
        case 'missing':
            return lucideSvg([
                h('path', { d: 'M4 22h14a2 2 0 0 0 2-2V7l-5-5H6a2 2 0 0 0-2 2v4' }),
                h('path', { d: 'M14 2v4a2 2 0 0 0 2 2h4' }),
                h('path', { d: 'm8 12.5-5 5' }),
                h('path', { d: 'm3 12.5 5 5' }),
            ]);
    }
}

function renderGroupHeader(row: HeaderRow): ReturnType<typeof h> {
    return h('div', { class: 'flex items-center gap-2 py-1' }, [
        h('i', { class: 'fa-solid fa-layer-group text-stone-400 text-xs' }),
        h(
            'span',
            { class: 'font-semibold text-xs uppercase tracking-wide text-stone-600 dark:text-stone-300' },
            row.title,
        ),
        h('span', { class: 'badge-stone !me-0 text-[10px]' }, String(row.count)),
    ]);
}

function renderComment(row: PluginRow): ReturnType<typeof h> | null {
    if (editingKey.value === row.key) {
        return h('input', {
            class: 'mt-0.5 w-full text-xs px-1.5 py-0.5 rounded border border-stone-300 dark:border-stone-600 bg-white dark:bg-stone-800 text-stone-700 dark:text-stone-200 focus:outline-none focus:ring-1 focus:ring-emerald-400',
            value: editText.value,
            placeholder: trans('comment_placeholder'),
            onInput: (event: Event) => {
                editText.value = (event.target as HTMLInputElement).value;
            },
            onBlur: () => saveComment(row),
            onKeydown: (event: KeyboardEvent) => {
                if (event.key === 'Enter') {
                    event.preventDefault();
                    (event.target as HTMLInputElement).blur();
                } else if (event.key === 'Escape') {
                    event.preventDefault();
                    cancelEdit();
                    (event.target as HTMLInputElement).blur();
                }
            },
            onVnodeMounted: (vnode) => {
                (vnode.el as HTMLInputElement | null)?.focus();
            },
        });
    }
    const text = row.comment
        ? h('span', { class: 'break-words min-w-0' }, row.comment)
        : h('span', { class: 'italic opacity-50' }, trans('comment_add'));
    const pencil = h(
        'button',
        {
            class: 'shrink-0 text-stone-400 hover:text-stone-600 dark:hover:text-stone-200 disabled:opacity-40',
            disabled: props.busy,
            title: trans('comment_edit'),
            onClick: () => startEdit(row),
        },
        h('i', { class: 'fa-solid fa-pen text-[10px]' }),
    );
    return h(
        'div',
        {
            class: 'mt-0.5 flex items-start gap-1.5 text-xs text-stone-500 dark:text-stone-400 min-w-0',
        },
        [text, pencil],
    );
}

function renderActionButton(
    color: string,
    icon: string,
    label: string,
    onClick: () => void,
): ReturnType<typeof h> {
    return h(
        'button',
        {
            class: `inline-flex items-center justify-center align-middle text-center select-none whitespace-nowrap rounded text-xs py-1.5 px-2 ${buttonClasses(color)}`,
            style: 'width: 6.5rem; flex: none;',
            disabled: props.busy,
            onClick,
        },
        [h('i', { class: icon }), h('span', { class: 'hidden lg:inline ml-1' }, label)],
    );
}

function buttonClasses(color: string): string {
    switch (color) {
        case 'red':
            return 'bg-red-500 text-white hover:bg-red-600 dark:bg-red-800 dark:hover:bg-red-900 dark:text-stone-200';
        default:
            return 'text-black bg-white hover:bg-stone-100 border border-stone-200 dark:border-stone-600 dark:bg-stone-800 dark:text-white dark:hover:bg-stone-700';
    }
}

const columns = computed<DataTableColumns<TableRow>>(() => {
    const cols: DataTableColumns<TableRow> = [
        {
            key: '__select',
            title: () =>
                h(NCheckbox, {
                    checked: allChecked.value,
                    indeterminate: someChecked.value,
                    'onUpdate:checked': (value: boolean) => toggleAll(value),
                }),
            width: 42,
            colSpan: (row: TableRow) => (isHeader(row) ? COLUMN_COUNT : 1),
            render(row: TableRow) {
                if (isHeader(row)) {
                    return renderGroupHeader(row);
                }
                return h(NCheckbox, {
                    checked: checked.value.includes(row.key),
                    'onUpdate:checked': (value: boolean) => toggleChecked(row.key, value),
                });
            },
        },
        {
            title: trans('col_plugin'),
            key: 'plugin',
            render(row: TableRow) {
                if (isHeader(row)) {
                    return null;
                }
                const meta = statusMeta(row);
                const circle = h(
                    'span',
                    {
                        class: `${meta.cls} !me-0 inline-flex items-center justify-center w-7 h-7 rounded-full flex-shrink-0`,
                    },
                    renderStatusIcon(row.status),
                );
                const icon = h(
                    NTooltip,
                    { trigger: 'hover' },
                    {
                        trigger: () => circle,
                        default: () =>
                            row.statusDetail ? `${meta.text} — ${row.statusDetail}` : meta.text,
                    },
                );
                return h('div', { class: 'flex items-center gap-3 min-w-0' }, [
                    icon,
                    h('div', { class: 'flex flex-col leading-tight min-w-0' }, [
                        h(
                            'span',
                            { class: 'font-medium text-stone-800 dark:text-stone-100' },
                            row.displayName,
                        ),
                        h(
                            'span',
                            { class: 'text-xs text-stone-500 dark:text-stone-400 font-mono truncate' },
                            row.name,
                        ),
                        renderComment(row),
                    ]),
                ]);
            },
        },
        {
            title: trans('col_version'),
            key: 'version',
            width: 120,
            render(row: TableRow) {
                if (isHeader(row)) {
                    return null;
                }
                const lines = [
                    h(
                        'span',
                        { class: 'text-sm text-stone-800 dark:text-stone-100 font-mono' },
                        row.version ? `v${row.version}` : '—',
                    ),
                ];
                if (row.author) {
                    lines.push(
                        h('span', { class: 'text-xs text-stone-500 dark:text-stone-400' }, row.author),
                    );
                }
                return h('div', { class: 'flex flex-col leading-tight' }, lines);
            },
        },
        {
            title: trans('col_enabled'),
            key: 'enabled',
            width: 56,
            align: 'center',
            render(row: TableRow) {
                if (isHeader(row)) {
                    return null;
                }
                return h(NSwitch, {
                    size: 'small',
                    value: row.enabled,
                    disabled: props.busy || row.missing,
                    'onUpdate:value': (value: boolean) => emit('toggle', row, value),
                });
            },
        },
        {
            title: trans('col_actions'),
            key: 'actions',
            align: 'right',
            width: 360,
            render(row: TableRow) {
                if (isHeader(row)) {
                    return null;
                }
                const spacer = () => h('span', { style: 'width: 6.5rem; flex: none;' });
                const buttons = [];
                const hotAction = row.missing ? null : hotActionForStatus(row.status, row.enabled);
                if (row.hasConfig) {
                    buttons.push(
                        renderActionButton('white', 'fa-solid fa-gear', trans('action_config'), () =>
                            emit('configure', row),
                        ),
                    );
                } else {
                    buttons.push(spacer());
                }
                if (hotAction === 'unload') {
                    buttons.push(
                        renderActionButton('white', 'fa-solid fa-pause', trans('action_unload'), () =>
                            emit('hot-action', row, 'unload'),
                        ),
                    );
                } else if (hotAction === 'load') {
                    buttons.push(
                        renderActionButton('white', 'fa-solid fa-play', trans('action_load'), () =>
                            emit('hot-action', row, 'load'),
                        ),
                    );
                } else {
                    buttons.push(spacer());
                }
                buttons.push(
                    renderActionButton('red', 'fa-solid fa-trash-can', trans('action_delete'), () =>
                        emit('remove', row),
                    ),
                );
                return h('div', { class: 'flex justify-end items-center gap-1' }, buttons);
            },
        },
    ];

    return cols;
});

function bulk(action: 'enable' | 'disable' | 'delete'): void {
    const selected = props.rows.filter((row) => checked.value.includes(row.key));
    if (selected.length === 0) {
        return;
    }
    emit('bulk', action, selected);
    checked.value = [];
}
</script>

<style scoped>
:deep(.csa-group-row > td) {
    background-color: rgba(120, 113, 108, 0.06);
}

@keyframes csa-heartbeat {
    0%,
    100% {
        transform: scale(1);
    }
    14% {
        transform: scale(1.25);
    }
    28% {
        transform: scale(1);
    }
    42% {
        transform: scale(1.25);
    }
    70% {
        transform: scale(1);
    }
}

:deep(.csa-heartbeat) {
    animation: csa-heartbeat 2.6s ease-in-out infinite;
    transform-origin: center;
}

@media (prefers-reduced-motion: reduce) {
    :deep(.csa-heartbeat) {
        animation: none;
    }
}
</style>
