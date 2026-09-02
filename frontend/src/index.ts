import type { PluginDefinition } from '@gameap/plugin-sdk';

import ModsTab from './components/ModsTab.vue';

export const cs2AddonsPlugin: PluginDefinition = {
    // Must match src/lib.rs PLUGIN_ID and survive the panel's CompactPluginID
    // normalization (valid base32, a-z2-7): base32("cs2addon").
    id: 'mnzteylemrxw4',
    name: 'CS2 Addons',
    version: '0.6.4',
    apiVersion: '1.0',
    description: 'Manage Metamod:Source and CounterStrikeSharp plugins on Counter-Strike 2 servers',
    author: 'SilverSasquatchGameAPDev',

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
            file_hint: 'A compiled plugin .dll, or a whole release .zip - the archive layout is detected and unpacked to the right place automatically.',
            wrong_type: 'A .dll or .zip file is required',
            zip_installed_toast: 'Installed from archive: :folders (:count files)',
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
            vdf_platform_badge: 'platform',
            vdf_platform_title: 'Disable CounterStrikeSharp itself?',
            vdf_platform_text:
                'This alias registers CounterStrikeSharp with Metamod. Switching it off unloads the whole platform at the next restart: every CSS plugin stops and this tab loses live statuses and hot load/unload. Disable it anyway?',
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
            admins_empty: 'No admins yet',
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

            rcon_usercon_missing:
                'RCON cannot work: the launch parameters are missing -usercon. Add it in Launch Settings and restart.',
            rcon_metamod_not_loaded:
                'The console works, but Metamod is not loaded in the running server - restart the server to load it.',
            rcon_css_not_loaded:
                'Metamod is loaded but CounterStrikeSharp is not, so the console does not know css_plugins. Live statuses and hot load/unload are unavailable. Check that addons/metamod/counterstrikesharp.vdf is enabled and restart the server.',

            action_reload: 'Reload',
            reloaded_ok: 'Plugin ":name" reloaded',
            reload_failed: 'Failed to reload ":name"',

            update_all: 'Update all (:count)',
            update_all_title: 'Update :count plugins?',
            update_all_text:
                'Each plugin is reinstalled from its latest GitHub release. A snapshot is taken automatically first.',
            update_all_done: 'Plugins updated: :count - restart or reload them',
            update_all_partial: 'Updated :count, failed: :failed',

            toolbar_doctor: 'Doctor',
            doctor_title: 'Setup health check',
            doctor_recheck: 'Re-check',
            doctor_all_ok: 'Everything looks healthy',
            doctor_summary: ':fails failed, :warns warnings',
            doctor_usercon_ok: '-usercon is present in the launch parameters',
            doctor_usercon_missing:
                'The launch parameters are missing -usercon; RCON cannot work without it',
            doctor_rcon_ok: 'Console reachable, live data flowing',
            doctor_check_cssloaded: 'CounterStrikeSharp loaded',
            doctor_css_loaded_ok: 'Loaded in the running server and answering css_plugins',
            doctor_css_not_loaded:
                'Installed on disk but not loaded in the running server - every row below shows its folder state only. Check addons/metamod/counterstrikesharp.vdf and restart.',
            doctor_check_usercon: 'Launch parameters',
            doctor_check_rcon: 'RCON console',
            doctor_check_metamod: 'Metamod:Source',
            doctor_check_gameinfo: 'gameinfo.gi wiring',
            doctor_check_css: 'CounterStrikeSharp',
            doctor_check_duplicates: 'Enabled/disabled conflicts',
            doctor_check_layout: 'Plugin folder layout',
            doctor_check_orphans: 'Manifest consistency',
            doctor_check_shared: 'Shared assemblies',
            doctor_check_stray: 'Plugin folder placement',
            doctor_check_loadfail: 'Plugin load failures',
            doctor_check_vdf: 'Metamod plugin aliases',
            doctor_check_scratch: 'Download leftovers',

            config_format: 'Format',
            config_invalid: 'Not valid JSON - saving is blocked: :error',

            logs_follow: 'Follow',
            logs_download: 'download',
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
            file_hint: 'Скомпилированный .dll плагина или целый релизный .zip - раскладка архива определяется и распаковывается куда нужно автоматически.',
            wrong_type: 'Нужен файл .dll или .zip',
            zip_installed_toast: 'Установлено из архива: :folders (файлов: :count)',
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
            vdf_platform_badge: 'платформа',
            vdf_platform_title: 'Отключить сам CounterStrikeSharp?',
            vdf_platform_text:
                'Этот alias регистрирует CounterStrikeSharp в Metamod. Его отключение выгрузит всю платформу при следующем перезапуске: все CSS-плагины остановятся, а вкладка потеряет живые статусы и горячую загрузку. Всё равно отключить?',
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
            admins_empty: 'Администраторов пока нет',
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

            rcon_usercon_missing:
                'RCON не может работать: в параметрах запуска нет -usercon. Добавьте его в настройках запуска и перезапустите сервер.',
            rcon_metamod_not_loaded:
                'Консоль работает, но Metamod не загружен на запущенном сервере - перезапустите сервер.',
            rcon_css_not_loaded:
                'Metamod загружен, а CounterStrikeSharp - нет, поэтому консоль не знает команду css_plugins. Живые статусы и горячая загрузка недоступны. Проверьте, что addons/metamod/counterstrikesharp.vdf включён, и перезапустите сервер.',

            action_reload: 'Перезагрузить',
            reloaded_ok: 'Плагин «:name» перезагружен',
            reload_failed: 'Не удалось перезагрузить «:name»',

            update_all: 'Обновить все (:count)',
            update_all_title: 'Обновить плагины (:count)?',
            update_all_text:
                'Каждый плагин переустанавливается из последнего релиза на GitHub. Перед этим автоматически создаётся снапшот.',
            update_all_done: 'Обновлено плагинов: :count - перезапустите или перезагрузите их',
            update_all_partial: 'Обновлено: :count, не удалось: :failed',

            toolbar_doctor: 'Диагностика',
            doctor_title: 'Проверка состояния',
            doctor_recheck: 'Проверить снова',
            doctor_all_ok: 'Всё в порядке',
            doctor_summary: 'ошибок: :fails, предупреждений: :warns',
            doctor_usercon_ok: '-usercon есть в параметрах запуска',
            doctor_usercon_missing:
                'В параметрах запуска нет -usercon; без него RCON не работает',
            doctor_rcon_ok: 'Консоль доступна, данные идут',
            doctor_check_cssloaded: 'CounterStrikeSharp загружен',
            doctor_css_loaded_ok: 'Загружен на запущенном сервере, отвечает на css_plugins',
            doctor_css_not_loaded:
                'Установлен на диске, но не загружен на запущенном сервере - строки ниже показывают только состояние папок. Проверьте addons/metamod/counterstrikesharp.vdf и перезапустите сервер.',
            doctor_check_usercon: 'Параметры запуска',
            doctor_check_rcon: 'Консоль RCON',
            doctor_check_metamod: 'Metamod:Source',
            doctor_check_gameinfo: 'Подключение в gameinfo.gi',
            doctor_check_css: 'CounterStrikeSharp',
            doctor_check_duplicates: 'Конфликты вкл/выкл',
            doctor_check_layout: 'Структура папок плагинов',
            doctor_check_orphans: 'Согласованность манифеста',
            doctor_check_shared: 'Общие сборки',
            doctor_check_stray: 'Расположение папок плагинов',
            doctor_check_loadfail: 'Ошибки загрузки плагинов',
            doctor_check_vdf: 'Алиасы плагинов Metamod',
            doctor_check_scratch: 'Остатки загрузок',

            config_format: 'Форматировать',
            config_invalid: 'Некорректный JSON - сохранение заблокировано: :error',

            logs_follow: 'Следить',
            logs_download: 'скачать',
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
                // Source 2 only. It has to be by game code, not engine:
                // Source 1 and Source 2 games share the engine string "Source",
                // GameCheck has no engine-version field, and the panel ORs
                // engines with codes - so listing the engine at all would let
                // every Source 1 server back in. The backend enforces the real
                // rule (engine source, version 2) on every route regardless.
                checkGame: {
                    codes: ['cs2'],
                },
            },
        ],
    },
};
