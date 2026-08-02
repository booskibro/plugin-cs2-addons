// Mirrors of the Rust backend DTOs (src/model.rs).

export interface StatePaths {
    gameinfo: string;
    metamod_dir: string;
    css_dir: string;
    css_plugins_dir: string;
    css_disabled_dir: string;
    css_configs_dir: string;
    meta_manifest: string;
}

export interface CssPluginEntry {
    name: string;
    enabled: boolean;
    missing: boolean;
    comment: string | null;
    group: string | null;
    has_config: boolean;
    config_path: string | null;
    group_index: number;
    group_title: string | null;
}

export interface MetamodState {
    installed: boolean;
    dir_present: boolean;
    gameinfo_wired: boolean;
}

export interface CssState {
    installed: boolean;
    plugins: CssPluginEntry[];
}

export interface StateResponse {
    server_id: number;
    game_code: string;
    engine: string;
    engine_version: string;
    game_dir: string;
    paths: StatePaths;
    metamod: MetamodState;
    css: CssState;
}

// Runtime info assembled from RCON output.

export interface PlatformVersion {
    build: string;
    version: string;
}

export interface RuntimePluginInfo {
    /** Module name as printed by css_plugins list. */
    name: string;
    version: string | null;
    author: string | null;
    /** Normalized runtime state. */
    status: 'running' | 'stopped' | 'error';
    rawStatus: string;
}

/** Row model the plugin table renders. */
export interface PluginRow {
    key: string;
    /** Plugin folder name — the stable identity. */
    name: string;
    /** Human-oriented name: runtime ModuleName, or prettified folder name. */
    displayName: string;
    version: string | null;
    author: string | null;
    /** Folder is in plugins/ (not parked in plugins/disabled/). */
    enabled: boolean;
    /** plugins_meta.json note; editable. */
    comment: string | null;
    missing: boolean;
    runtime: RuntimePluginInfo | null;
    hasConfig: boolean;
    configPath: string | null;
    status: RowStatus;
    statusDetail: string | null;
    /** Display group id; ungrouped entries share one trailing "Other" group. */
    groupIndex: number;
    /** Display group header, `null` for the common "Other" group. */
    groupTitle: string | null;
}

export type RowStatus =
    | 'running'
    | 'enabled'
    | 'stopped'
    | 'pending'
    | 'error'
    | 'missing';

// Local mirror of the SDK's ServerData / ServerTabProps contract.
//
// Declared here rather than imported from @gameap/plugin-sdk so that
// `defineProps<ServerTabProps>()` compiles against the shipped plugin build: the
// Vue SFC compiler statically resolves the props type at build time, and the CI
// SDK build runs vite only (its `tsc --emitDeclarationOnly` step fails on
// @gameap/ui and emits no declarations), so the SDK's types are not on disk to
// resolve against.

export interface ServerData {
    id: number;
    uuid: string;
    name: string;
    game_id: string;
    game_mod_id: number;
    ip: string;
    port: number;
    query_port: number;
    rcon_port: number;
    enabled: boolean;
    installed: boolean;
    blocked: boolean;
    start_command: string;
    dir: string;
    process_active: boolean;
    last_process_check: string;
}

export interface ServerTabProps {
    serverId: number;
    server: ServerData;
    pluginId: string;
}
