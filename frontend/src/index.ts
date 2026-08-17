import type { PluginDefinition } from '@gameap/plugin-sdk';

import ModsTab from './components/ModsTab.vue';

export const cs2AddonsPlugin: PluginDefinition = {
    // Must match src/lib.rs PLUGIN_ID and survive the panel's CompactPluginID
    // normalization (valid base32, a-z2-7): base32("cs2addon").
    id: 'mnzteylemrxw4',
    name: 'CS2 Addons',
    version: '0.2.1',
    apiVersion: '1.0',
    description: 'Manage Metamod:Source and CounterStrikeSharp plugins on Counter-Strike 2 servers',
    author: 'BooskiBro (after gameap/plugin-goldsrc-addons by GameAP)',

    translations: {
        en: {
            tab_label: 'Plugins',
            'abilities.manage': 'Manage CS2 addons (Metamod:Source / CounterStrikeSharp)',

            metamod_desc:
                'Modification layer for the Source 2 engine. Loads binary plugins and is required by CounterStrikeSharp.',
            css_desc:
                'C# scripting platform on top of Metamod:Source: admin tools, match systems and gameplay plugins.',
            status_not_installed: 'Not installed',
            status_not_active: 'Not active',
            not_active_hint:
                'The addons/metamod directory exists, but gameinfo.gi does not load Metamod.',
            version_unknown: 'version unknown',
            stats_total: 'Plugins',
            stats_enabled: 'Enabled',
            stats_errors: 'Errors',
            install_hint_metamod:
                'Install Metamod:Source into addons/metamod and add the search path to gameinfo.gi.',
            install_hint_css:
                'Install CounterStrikeSharp (with runtime) into addons/counterstrikesharp.',

            rcon_unavailable_offline:
                'Server is offline — console versions and statuses are unavailable.',
            rcon_unavailable_norcon:
                'RCON is not configured — console versions and statuses are unavailable.',
            rcon_unavailable_error: 'Failed to query the server console.',
            rcon_unavailable_badpass:
                'Wrong RCON password — console versions and statuses are unavailable. Check the password in the server settings.',
            rcon_unavailable_empty:
                'The server console returned an empty response — versions and statuses are unavailable.',

            upload_file: 'Upload file',
            search_placeholder: 'Search by name, folder, author…',
            filter_all: 'All statuses',
            filter_on: 'Enabled',
            filter_off: 'Disabled',
            filter_err: 'With errors',
            selected: 'Selected: :count',
            bulk_enable: 'Enable',
            bulk_disable: 'Disable',
            bulk_delete: 'Delete',

            col_plugin: 'Plugin',
            col_version: 'Version',
            col_status: 'Status',
            col_enabled: 'On',
            col_actions: 'Actions',

            status_running: 'Running',
            status_enabled: 'Enabled',
            status_stopped: 'Stopped',
            status_pending: 'Awaiting load',
            status_error: 'Error',
            status_missing: 'Files missing',

            action_config: 'Config',
            action_delete: 'Delete',
            action_unload: 'Unload',
            action_load: 'Load',
            group_other: 'Other',
            comment_add: 'add comment',
            comment_edit: 'Edit comment',
            comment_placeholder: 'Comment…',
            comment_saved: 'Comment for ":name" saved',

            empty_no_plugins: 'No plugins installed',
            empty_no_results: 'Nothing found — adjust the search',
            install_first: 'Install the first plugin',
            open_in_filemanager: 'open in file manager',

            css_missing: 'CounterStrikeSharp is not installed',
            metamod_missing: 'Metamod:Source is not installed',
            platform_missing_hint:
                'Install the platform on the server — the plugin list and file upload will appear here.',
            nothing_installed_title: 'Metamod:Source and CounterStrikeSharp are not installed',
            nothing_installed_text:
                'Install Metamod:Source and CounterStrikeSharp on the server to manage plugins from the panel: enable, disable and upload your own files.',
            not_source2:
                'This tab is available only for Source 2 servers (Counter-Strike 2).',

            loading: 'Loading…',
            load_failed: 'Failed to load the plugins state',
            retry: 'Retry',

            delete_title: 'Delete plugin ":name"?',
            delete_text:
                'The plugin folder will be removed. Configs in configs/plugins are kept.',
            bulk_delete_title: 'Delete selected plugins (:count)?',
            bulk_delete_text: 'Plugin folders are removed; configs are kept.',
            yes: 'Yes',
            no: 'No',

            toggled_on: 'Plugin ":name" enabled — load it or restart the server',
            toggled_off: 'Plugin ":name" disabled — applies after restart or unload',
            unloaded_ok: 'Plugin ":name" unloaded',
            loaded_ok: 'Plugin ":name" loaded',
            unload_failed: 'Failed to unload ":name"',
            load_failed_named: 'Failed to load ":name"',
            deleted: 'Plugin ":name" deleted',
            bulk_enabled: 'Plugins enabled: :count',
            bulk_disabled: 'Plugins disabled: :count',
            bulk_deleted: 'Plugins deleted: :count',
            installed_toast: 'Plugin ":name" installed from file',
            op_failed: 'Operation failed',

            install_title: 'Install plugin — CounterStrikeSharp',
            drop_hint: 'Drop a file here or click to choose',
            file_hint: 'A compiled plugin .dll is supported; the folder plugins/<Name>/ is created automatically. Multi-file plugins: unpack their zip via the file manager, then register with Upload.',
            wrong_type: 'A .dll file is required',
            auto_enable: 'Load after install',
            install: 'Install',
            uploading: 'Uploading…',
            overwrite: 'Overwrite',
            overwrite_title: 'Plugin already installed',
            overwrite_text:
                'The existing .dll will be overwritten with the new version; comments and configs stay in place.',
            updated_toast: 'Plugin ":name" updated from file',

            config_title: 'Configuration — :name',
            save: 'Save',
            config_saved: 'Configuration saved',
            config_load_failed: 'Failed to load the config',

            toolbar_catalog: 'Catalog',
            toolbar_snapshots: 'Snapshots',
            toolbar_admins: 'Admins',
            toolbar_logs: 'Logs',
            toolbar_history: 'History',

            update_available: 'v:version available',
            update_available_hint: 'A newer release is available upstream',

            repair_gameinfo: 'Fix gameinfo.gi',
            gameinfo_repaired: 'gameinfo.gi patched - the Metamod search path is back',
            gameinfo_already_ok: 'gameinfo.gi already loads Metamod - nothing to fix',

            vdf_plugins: 'Metamod plugins',
            vdf_hint: 'Toggling renames the .vdf; applies on the next restart.',
            vdf_enabled: 'Metamod plugin ":name" enabled - applies after restart',
            vdf_disabled: 'Metamod plugin ":name" disabled - applies after restart',

            platform_install: 'Install latest',
            platform_installing: 'Installing…',
            platform_update: 'Update to v:version',
            platform_install_title: 'Install :name?',
            platform_install_text:
                'The latest release is downloaded and unpacked on the server (existing files are overwritten). This can take a minute.',
            platform_installed: ':name :version installed - restart the server to load it',

            restart_pending: 'Changes are waiting for a server restart to take effect.',
            restart_now: 'Restart server',
            restart_title: 'Restart the server?',
            restart_text: 'Players online will be disconnected.',
            restart_sent: 'Restart requested',

            catalog_title: 'Plugin catalog',
            catalog_installed: 'installed',
            catalog_installing: 'Installing…',
            catalog_reinstall: 'Reinstall',
            catalog_hint:
                'Installs the latest GitHub release. Multi-server configs and databases still need per-plugin setup - check each project page.',
            catalog_installed_toast: ':name :version installed - load it or restart the server',

            snapshots_title: 'Plugin setup snapshots',
            snapshot_create: 'Create snapshot',
            snapshots_retention: 'plugins/ + configs are archived; the 5 newest are kept.',
            snapshots_empty: 'No snapshots yet',
            snapshot_download: 'download',
            snapshot_restore: 'Restore',
            snapshot_created: 'Snapshot created',
            snapshot_restore_title: 'Restore this snapshot?',
            snapshot_restore_text:
                'Plugins and configs are replaced with the state from :date. Files added since then are removed.',
            snapshot_restored: 'Snapshot restored - restart the server to apply',
            snapshots_transfer_hint:
                'To copy a setup between servers: download a snapshot here, upload the .tar into the other server’s backups folder via its file manager, then restore it there.',

            admins_title: 'CounterStrikeSharp admins',
            admins_tab_admins: 'Admins',
            admins_tab_groups: 'Groups (raw JSON)',
            admins_col_name: 'Name',
            admins_col_identity: 'SteamID64',
            admins_col_flags: 'Flags',
            admins_col_immunity: 'Immunity',
            admins_add: 'Add admin',
            admins_hint:
                'Flags are comma-separated (e.g. @css/generic, @css/ban). Changes apply on map change or css_admins_reload.',
            admins_parse_failed: 'admins.json could not be parsed - fix it via the file manager',
            admins_groups_invalid: 'admin_groups.json is not valid JSON: :error',
            admins_saved: 'Admin configuration saved',

            logs_title: 'CounterStrikeSharp log',
            logs_filter_placeholder: 'Filter lines (plugin name, "error"…)',
            logs_empty: 'No log lines - the server has not written a CSS log yet',

            audit_title: 'Recent panel actions',
            audit_empty: 'Nothing recorded yet',
        },
        ru: {
            tab_label: 'Плагины',
            'abilities.manage': 'Управление CS2-аддонами (Metamod:Source / CounterStrikeSharp)',

            metamod_desc:
                'Слой модификаций для движка Source 2. Загружает бинарные плагины и требуется для работы CounterStrikeSharp.',
            css_desc:
                'C#-платформа поверх Metamod:Source: администрирование, матч-системы и геймплейные плагины.',
            status_not_installed: 'Не установлен',
            status_not_active: 'Не активен',
            not_active_hint:
                'Каталог addons/metamod найден, но gameinfo.gi не подключает Metamod.',
            version_unknown: 'версия неизвестна',
            stats_total: 'Плагинов',
            stats_enabled: 'Включено',
            stats_errors: 'Ошибок',
            install_hint_metamod:
                'Установите Metamod:Source в addons/metamod и добавьте путь в gameinfo.gi.',
            install_hint_css:
                'Установите CounterStrikeSharp (сборку с рантаймом) в addons/counterstrikesharp.',

            rcon_unavailable_offline:
                'Сервер офлайн — версии и статусы из консоли недоступны.',
            rcon_unavailable_norcon:
                'RCON не настроен — версии и статусы из консоли недоступны.',
            rcon_unavailable_error: 'Не удалось опросить консоль сервера.',
            rcon_unavailable_badpass:
                'Неверный RCON-пароль — версии и статусы из консоли недоступны. Проверьте пароль в настройках сервера.',
            rcon_unavailable_empty:
                'Консоль сервера вернула пустой ответ — версии и статусы недоступны.',

            upload_file: 'Загрузить файл',
            search_placeholder: 'Поиск по названию, папке, автору…',
            filter_all: 'Все статусы',
            filter_on: 'Включённые',
            filter_off: 'Выключенные',
            filter_err: 'С ошибками',
            selected: 'Выбрано: :count',
            bulk_enable: 'Включить',
            bulk_disable: 'Выключить',
            bulk_delete: 'Удалить',

            col_plugin: 'Плагин',
            col_version: 'Версия',
            col_status: 'Статус',
            col_enabled: 'Вкл.',
            col_actions: 'Действия',

            status_running: 'Работает',
            status_enabled: 'Включен',
            status_stopped: 'Остановлен',
            status_pending: 'Ждёт загрузки',
            status_error: 'Ошибка',
            status_missing: 'Файлы отсутствуют',

            action_config: 'Конфиг',
            action_delete: 'Удалить',
            action_unload: 'Выгрузить',
            action_load: 'Загрузить',
            group_other: 'Прочее',
            comment_add: 'добавить комментарий',
            comment_edit: 'Изменить комментарий',
            comment_placeholder: 'Комментарий…',
            comment_saved: 'Комментарий для «:name» сохранён',

            empty_no_plugins: 'Плагины не установлены',
            empty_no_results: 'Ничего не найдено — измените условия поиска',
            install_first: 'Установить первый плагин',
            open_in_filemanager: 'открыть в файловом менеджере',

            css_missing: 'CounterStrikeSharp не установлен',
            metamod_missing: 'Metamod:Source не установлен',
            platform_missing_hint:
                'Установите платформу на сервере — после этого здесь появится список плагинов и загрузка файлов.',
            nothing_installed_title: 'Metamod:Source и CounterStrikeSharp не установлены',
            nothing_installed_text:
                'Установите Metamod:Source и CounterStrikeSharp на сервер — и управляйте плагинами прямо из панели: включение, выключение и загрузка своих файлов.',
            not_source2:
                'Вкладка доступна только для серверов на движке Source 2 (Counter-Strike 2).',

            loading: 'Загрузка…',
            load_failed: 'Не удалось загрузить состояние плагинов',
            retry: 'Повторить',

            delete_title: 'Удалить плагин «:name»?',
            delete_text:
                'Папка плагина будет удалена. Конфигурация в configs/plugins сохранится.',
            bulk_delete_title: 'Удалить выбранные плагины (:count)?',
            bulk_delete_text: 'Папки плагинов удаляются; конфигурация сохраняется.',
            yes: 'Да',
            no: 'Нет',

            toggled_on: 'Плагин «:name» включён — загрузите его или перезапустите сервер',
            toggled_off: 'Плагин «:name» выключен — применится после перезапуска или выгрузки',
            unloaded_ok: 'Плагин «:name» выгружен',
            loaded_ok: 'Плагин «:name» загружен',
            unload_failed: 'Не удалось выгрузить «:name»',
            load_failed_named: 'Не удалось загрузить «:name»',
            deleted: 'Плагин «:name» удалён',
            bulk_enabled: 'Включено плагинов: :count',
            bulk_disabled: 'Выключено плагинов: :count',
            bulk_deleted: 'Удалено плагинов: :count',
            installed_toast: 'Плагин «:name» установлен из файла',
            op_failed: 'Операция не выполнена',

            install_title: 'Установка плагина — CounterStrikeSharp',
            drop_hint: 'Перетащите файл сюда или нажмите для выбора',
            file_hint: 'Поддерживается скомпилированный .dll плагина; папка plugins/<Name>/ создаётся автоматически. Многофайловые плагины: распакуйте их zip через файловый менеджер и зарегистрируйте кнопкой «Загрузить».',
            wrong_type: 'Нужен файл .dll',
            auto_enable: 'Загрузить после установки',
            install: 'Установить',
            uploading: 'Загрузка…',
            overwrite: 'Перезаписать',
            overwrite_title: 'Плагин уже установлен',
            overwrite_text:
                'Существующий .dll будет перезаписан новой версией; комментарии и конфиги сохранятся.',
            updated_toast: 'Плагин «:name» обновлён из файла',

            config_title: 'Конфигурация — :name',
            save: 'Сохранить',
            config_saved: 'Конфигурация сохранена',
            config_load_failed: 'Не удалось загрузить конфиг',

            toolbar_catalog: 'Каталог',
            toolbar_snapshots: 'Снапшоты',
            toolbar_admins: 'Админы',
            toolbar_logs: 'Логи',
            toolbar_history: 'История',

            update_available: 'доступна v:version',
            update_available_hint: 'Вышла более новая версия',

            repair_gameinfo: 'Починить gameinfo.gi',
            gameinfo_repaired: 'gameinfo.gi исправлен - путь Metamod возвращён',
            gameinfo_already_ok: 'gameinfo.gi уже загружает Metamod - чинить нечего',

            vdf_plugins: 'Плагины Metamod',
            vdf_hint: 'Переключение переименовывает .vdf; применится после перезапуска.',
            vdf_enabled: 'Metamod-плагин «:name» включён - применится после перезапуска',
            vdf_disabled: 'Metamod-плагин «:name» выключен - применится после перезапуска',

            platform_install: 'Установить последнюю',
            platform_installing: 'Установка…',
            platform_update: 'Обновить до v:version',
            platform_install_title: 'Установить :name?',
            platform_install_text:
                'Последний релиз будет скачан и распакован на сервере (существующие файлы перезаписываются). Это может занять минуту.',
            platform_installed: ':name :version установлен - перезапустите сервер',

            restart_pending: 'Изменения ждут перезапуска сервера.',
            restart_now: 'Перезапустить',
            restart_title: 'Перезапустить сервер?',
            restart_text: 'Игроки на сервере будут отключены.',
            restart_sent: 'Перезапуск запрошен',

            catalog_title: 'Каталог плагинов',
            catalog_installed: 'установлен',
            catalog_installing: 'Установка…',
            catalog_reinstall: 'Переустановить',
            catalog_hint:
                'Устанавливается последний релиз с GitHub. Базы данных и сложные конфиги настраиваются отдельно - смотрите страницу проекта.',
            catalog_installed_toast: ':name :version установлен - загрузите его или перезапустите сервер',

            snapshots_title: 'Снапшоты набора плагинов',
            snapshot_create: 'Создать снапшот',
            snapshots_retention: 'Архивируются plugins/ и конфиги; хранятся 5 последних.',
            snapshots_empty: 'Снапшотов пока нет',
            snapshot_download: 'скачать',
            snapshot_restore: 'Восстановить',
            snapshot_created: 'Снапшот создан',
            snapshot_restore_title: 'Восстановить этот снапшот?',
            snapshot_restore_text:
                'Плагины и конфиги будут заменены состоянием от :date. Файлы, добавленные позже, будут удалены.',
            snapshot_restored: 'Снапшот восстановлен - перезапустите сервер',
            snapshots_transfer_hint:
                'Чтобы перенести набор на другой сервер: скачайте снапшот, загрузите .tar в папку backups другого сервера через его файловый менеджер и восстановите там.',

            admins_title: 'Админы CounterStrikeSharp',
            admins_tab_admins: 'Админы',
            admins_tab_groups: 'Группы (JSON)',
            admins_col_name: 'Имя',
            admins_col_identity: 'SteamID64',
            admins_col_flags: 'Флаги',
            admins_col_immunity: 'Иммунитет',
            admins_add: 'Добавить админа',
            admins_hint:
                'Флаги через запятую (например @css/generic, @css/ban). Применяется на смене карты или по css_admins_reload.',
            admins_parse_failed: 'admins.json не разбирается - исправьте его через файловый менеджер',
            admins_groups_invalid: 'admin_groups.json - некорректный JSON: :error',
            admins_saved: 'Настройки админов сохранены',

            logs_title: 'Лог CounterStrikeSharp',
            logs_filter_placeholder: 'Фильтр строк (имя плагина, «error»…)',
            logs_empty: 'Строк нет - сервер ещё не писал лог CSS',

            audit_title: 'Последние действия в панели',
            audit_empty: 'Пока ничего не записано',
        },
    },

    slots: {
        'server-tabs': [
            {
                component: ModsTab,
                order: 100,
                label: '@:tab_label',
                icon: 'plug',
                name: 'plugins',
                checkPermission: {
                    type: 'hasServerPermissions',
                    permissions: ['plugin:mnzteylemrxw4:manage'],
                },
                checkGame: {
                    engines: ['Source'],
                },
            },
        ],
    },
};
