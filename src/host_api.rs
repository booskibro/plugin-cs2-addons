//! Seam between handlers and the SDK host functions.
//!
//! `gameap_plugin_sdk::host` is compiled only for wasm32, so handlers depend
//! on this trait instead; native `cargo test` runs them against `MockHost`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostApiError {
    /// ABI/transport failure of the host call itself.
    Call(String),
    /// The daemon reported a failed operation (`success: false` / `error`).
    Op(String),
}

pub type HostResult<T> = Result<T, HostApiError>;

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub id: u64,
    pub game_code: String,
    pub node_id: u64,
    pub dir: String,
}

#[derive(Debug, Clone)]
pub struct GameInfo {
    pub code: String,
    pub name: String,
    pub engine: String,
    pub engine_version: String,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: u64,
    pub os: String,
    pub work_path: String,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct FileStat {
    pub is_dir: bool,
    pub size: u64,
    /// Unix permission bits (0 when the node does not report them).
    pub permissions: u32,
}

pub trait HostApi {
    fn get_server(&mut self, id: u64) -> HostResult<Option<ServerInfo>>;
    fn get_game(&mut self, code: &str) -> HostResult<Option<GameInfo>>;
    fn get_node(&mut self, id: u64) -> HostResult<Option<NodeInfo>>;
    /// `Ok(None)` — directory missing or unreadable.
    fn read_dir(&mut self, node_id: u64, path: &str) -> HostResult<Option<Vec<DirEntry>>>;
    /// Existence probe; `Ok(None)` — not found.
    fn stat(&mut self, node_id: u64, path: &str) -> HostResult<Option<FileStat>>;
    fn download(&mut self, node_id: u64, path: &str) -> HostResult<Vec<u8>>;
    fn upload(&mut self, node_id: u64, path: &str, content: &[u8], permissions: u32)
    -> HostResult<()>;
    fn remove(&mut self, node_id: u64, path: &str, recursive: bool) -> HostResult<()>;
    fn chmod(&mut self, node_id: u64, path: &str, permissions: u32) -> HostResult<()>;
    fn mk_dir(&mut self, node_id: u64, path: &str) -> HostResult<()>;
    /// Rename/move a file or directory. The daemon's nodecmd has NO shell
    /// (commands are shellquote-split and exec'd directly), so folder moves
    /// must use this native op — never `mv` through execute_command.
    fn move_path(&mut self, node_id: u64, source: &str, destination: &str) -> HostResult<()>;
    fn log_info(&mut self, message: &str);
    fn log_error(&mut self, message: &str);
}

pub struct WasmHost;

/// HashMap-backed fake node used by native handler tests.
#[cfg(test)]
pub mod mock {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[derive(Default)]
    pub struct MockHost {
        pub servers: BTreeMap<u64, ServerInfo>,
        pub games: BTreeMap<String, GameInfo>,
        pub nodes: BTreeMap<u64, NodeInfo>,
        /// Absolute path → file content.
        pub files: BTreeMap<String, Vec<u8>>,
        /// Absolute path → unix permission bits (files default to 0o644).
        pub perms: BTreeMap<String, u32>,
        /// Absolute paths of directories.
        pub dirs: BTreeSet<String>,
        /// chmod calls, in call order.
        pub chmods: Vec<(String, u32)>,
        /// move_path calls, in call order.
        pub moves: Vec<(String, String)>,
        pub logs: Vec<String>,
    }

    impl MockHost {
        /// A CS2 server (id 3) on node 1 rooted at /srv/gameap/servers/cs2.
        pub fn cs2() -> MockHost {
            let mut host = MockHost::default();
            host.servers.insert(
                3,
                ServerInfo {
                    id: 3,
                    game_code: "cs2".into(),
                    node_id: 1,
                    dir: "servers/cs2".into(),
                },
            );
            host.games.insert(
                "cs2".into(),
                GameInfo {
                    code: "cs2".into(),
                    name: "Counter-Strike 2".into(),
                    engine: "source".into(),
                    engine_version: "2".into(),
                },
            );
            host.nodes.insert(
                1,
                NodeInfo {
                    id: 1,
                    os: "linux".into(),
                    work_path: "/srv/gameap".into(),
                },
            );
            host.add_dir(MockHost::GAME_ABS);
            host.add_file(
                &format!("{}/gameinfo.gi", MockHost::GAME_ABS),
                b"\"GameInfo\"\n{\n\tgame \"Counter-Strike 2\"\n}\n",
            );
            host
        }

        pub const GAME_ABS: &str = "/srv/gameap/servers/cs2/game/csgo";

        pub fn add_dir(&mut self, path: &str) {
            let mut current = String::new();
            for segment in path.trim_start_matches('/').split('/') {
                current.push('/');
                current.push_str(segment);
                self.dirs.insert(current.clone());
            }
        }

        pub fn add_file(&mut self, path: &str, content: &[u8]) {
            if let Some(idx) = path.rfind('/') {
                self.add_dir(&path[..idx]);
            }
            self.files.insert(path.to_string(), content.to_vec());
        }

        pub fn file(&self, path: &str) -> Option<&[u8]> {
            self.files.get(path).map(Vec::as_slice)
        }

        pub fn set_perms(&mut self, path: &str, permissions: u32) {
            self.perms.insert(path.to_string(), permissions);
        }
    }

    impl HostApi for MockHost {
        fn get_server(&mut self, id: u64) -> HostResult<Option<ServerInfo>> {
            Ok(self.servers.get(&id).cloned())
        }

        fn get_game(&mut self, code: &str) -> HostResult<Option<GameInfo>> {
            Ok(self.games.get(code).cloned())
        }

        fn get_node(&mut self, id: u64) -> HostResult<Option<NodeInfo>> {
            Ok(self.nodes.get(&id).cloned())
        }

        fn read_dir(&mut self, _node_id: u64, path: &str) -> HostResult<Option<Vec<DirEntry>>> {
            let path = path.trim_end_matches('/');
            if !self.dirs.contains(path) {
                return Ok(None);
            }
            let prefix = format!("{path}/");
            let mut entries = Vec::new();
            for dir in &self.dirs {
                if let Some(rest) = dir.strip_prefix(&prefix)
                    && !rest.is_empty()
                    && !rest.contains('/')
                {
                    entries.push(DirEntry {
                        name: rest.to_string(),
                        is_dir: true,
                    });
                }
            }
            for file in self.files.keys() {
                if let Some(rest) = file.strip_prefix(&prefix)
                    && !rest.is_empty()
                    && !rest.contains('/')
                {
                    entries.push(DirEntry {
                        name: rest.to_string(),
                        is_dir: false,
                    });
                }
            }
            Ok(Some(entries))
        }

        fn stat(&mut self, _node_id: u64, path: &str) -> HostResult<Option<FileStat>> {
            let path = path.trim_end_matches('/');
            if let Some(content) = self.files.get(path) {
                return Ok(Some(FileStat {
                    is_dir: false,
                    size: content.len() as u64,
                    permissions: self.perms.get(path).copied().unwrap_or(0o644),
                }));
            }
            if self.dirs.contains(path) {
                return Ok(Some(FileStat {
                    is_dir: true,
                    size: 0,
                    permissions: 0o755,
                }));
            }
            Ok(None)
        }

        fn download(&mut self, _node_id: u64, path: &str) -> HostResult<Vec<u8>> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| HostApiError::Op(format!("no such file: {path}")))
        }

        fn upload(
            &mut self,
            _node_id: u64,
            path: &str,
            content: &[u8],
            _permissions: u32,
        ) -> HostResult<()> {
            self.add_file(path, content);
            Ok(())
        }

        fn remove(&mut self, _node_id: u64, path: &str, recursive: bool) -> HostResult<()> {
            let path = path.trim_end_matches('/');
            if self.files.remove(path).is_some() {
                return Ok(());
            }
            if self.dirs.contains(path) {
                if !recursive {
                    return Err(HostApiError::Op(format!("directory not empty: {path}")));
                }
                let prefix = format!("{path}/");
                self.dirs
                    .retain(|dir| dir != path && !dir.starts_with(&prefix));
                self.files.retain(|file, _| !file.starts_with(&prefix));
                return Ok(());
            }
            Err(HostApiError::Op(format!("no such file: {path}")))
        }

        fn chmod(&mut self, _node_id: u64, path: &str, permissions: u32) -> HostResult<()> {
            if !self.files.contains_key(path) && !self.dirs.contains(path) {
                return Err(HostApiError::Op(format!("no such file: {path}")));
            }
            self.chmods.push((path.to_string(), permissions));
            self.perms.insert(path.to_string(), permissions);
            Ok(())
        }

        fn mk_dir(&mut self, _node_id: u64, path: &str) -> HostResult<()> {
            self.add_dir(path);
            Ok(())
        }

        fn move_path(&mut self, _node_id: u64, source: &str, destination: &str) -> HostResult<()> {
            let source = source.trim_end_matches('/');
            let destination = destination.trim_end_matches('/');
            if !self.dirs.contains(source) && !self.files.contains_key(source) {
                return Err(HostApiError::Op(format!("no such file: {source}")));
            }
            self.moves.push((source.to_string(), destination.to_string()));
            if let Some(content) = self.files.remove(source) {
                self.add_file(destination, &content);
                return Ok(());
            }
            let src_prefix = format!("{source}/");
            let moved_dirs: Vec<String> = self
                .dirs
                .iter()
                .filter(|dir| *dir == source || dir.starts_with(&src_prefix))
                .cloned()
                .collect();
            for dir in moved_dirs {
                self.dirs.remove(&dir);
                self.dirs
                    .insert(format!("{destination}{}", &dir[source.len()..]));
            }
            let moved_files: Vec<String> = self
                .files
                .keys()
                .filter(|file| file.starts_with(&src_prefix))
                .cloned()
                .collect();
            for file in moved_files {
                if let Some(content) = self.files.remove(&file) {
                    self.files
                        .insert(format!("{destination}{}", &file[source.len()..]), content);
                }
            }
            Ok(())
        }

        fn log_info(&mut self, message: &str) {
            self.logs.push(format!("INFO {message}"));
        }

        fn log_error(&mut self, message: &str) {
            self.logs.push(format!("ERROR {message}"));
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use gameap_plugin_sdk::host;
    use gameap_plugin_sdk::proto::gameap::plugin::sdk::{games, nodefs, nodes, servers};

    use super::*;

    fn call_err(err: gameap_plugin_sdk::HostError) -> HostApiError {
        HostApiError::Call(err.to_string())
    }

    impl HostApi for WasmHost {
        fn get_server(&mut self, id: u64) -> HostResult<Option<ServerInfo>> {
            let resp = host::servers::get_server(&servers::GetServerRequest { id })
                .map_err(call_err)?;
            Ok(resp
                .found
                .then_some(resp.server)
                .flatten()
                .map(|s| ServerInfo {
                    id: s.id,
                    game_code: s.game_id,
                    node_id: s.ds_id,
                    dir: s.dir,
                }))
        }

        fn get_game(&mut self, code: &str) -> HostResult<Option<GameInfo>> {
            let resp = host::games::get_game(&games::GetGameRequest {
                code: code.to_owned(),
            })
            .map_err(call_err)?;
            Ok(resp.found.then_some(resp.game).flatten().map(|g| GameInfo {
                code: g.code,
                name: g.name,
                engine: g.engine,
                engine_version: g.engine_version,
            }))
        }

        fn get_node(&mut self, id: u64) -> HostResult<Option<NodeInfo>> {
            let resp = host::nodes::get_node(&nodes::GetNodeRequest { id }).map_err(call_err)?;
            Ok(resp.found.then_some(resp.node).flatten().map(|n| NodeInfo {
                id: n.id,
                os: n.os,
                work_path: n.work_path,
            }))
        }

        fn read_dir(&mut self, node_id: u64, path: &str) -> HostResult<Option<Vec<DirEntry>>> {
            let resp = host::nodefs::read_dir(&nodefs::ReadDirRequest {
                node_id,
                path: path.to_owned(),
            })
            .map_err(call_err)?;
            if resp.error.is_some() {
                return Ok(None);
            }
            let dir_type = nodefs::FileType::Dir as i32;
            Ok(Some(
                resp.files
                    .into_iter()
                    .map(|f| DirEntry {
                        name: f.name,
                        is_dir: f.r#type == dir_type,
                    })
                    .collect(),
            ))
        }

        fn stat(&mut self, node_id: u64, path: &str) -> HostResult<Option<FileStat>> {
            let resp = host::nodefs::get_file_info(&nodefs::GetFileInfoRequest {
                node_id,
                path: path.to_owned(),
            })
            .map_err(call_err)?;
            let dir_type = nodefs::FileType::Dir as i32;
            Ok(resp.found.then_some(resp.file).flatten().map(|f| FileStat {
                is_dir: f.r#type == dir_type,
                size: f.size,
                permissions: f.permissions,
            }))
        }

        fn download(&mut self, node_id: u64, path: &str) -> HostResult<Vec<u8>> {
            let resp = host::nodefs::download(&nodefs::DownloadRequest {
                node_id,
                path: path.to_owned(),
            })
            .map_err(call_err)?;
            match resp.error {
                Some(err) => Err(HostApiError::Op(err)),
                None => Ok(resp.content),
            }
        }

        fn upload(
            &mut self,
            node_id: u64,
            path: &str,
            content: &[u8],
            permissions: u32,
        ) -> HostResult<()> {
            let resp = host::nodefs::upload(&nodefs::UploadRequest {
                node_id,
                path: path.to_owned(),
                content: content.to_vec(),
                permissions,
            })
            .map_err(call_err)?;
            if resp.success {
                Ok(())
            } else {
                Err(HostApiError::Op(resp.error.unwrap_or_default()))
            }
        }

        fn remove(&mut self, node_id: u64, path: &str, recursive: bool) -> HostResult<()> {
            let resp = host::nodefs::remove(&nodefs::RemoveRequest {
                node_id,
                path: path.to_owned(),
                recursive,
            })
            .map_err(call_err)?;
            if resp.success {
                Ok(())
            } else {
                Err(HostApiError::Op(resp.error.unwrap_or_default()))
            }
        }

        fn chmod(&mut self, node_id: u64, path: &str, permissions: u32) -> HostResult<()> {
            let resp = host::nodefs::chmod(&nodefs::ChmodRequest {
                node_id,
                path: path.to_owned(),
                permissions,
            })
            .map_err(call_err)?;
            if resp.success {
                Ok(())
            } else {
                Err(HostApiError::Op(resp.error.unwrap_or_default()))
            }
        }

        fn mk_dir(&mut self, node_id: u64, path: &str) -> HostResult<()> {
            let resp = host::nodefs::mk_dir(&nodefs::MkDirRequest {
                node_id,
                path: path.to_owned(),
            })
            .map_err(call_err)?;
            if resp.success {
                Ok(())
            } else {
                Err(HostApiError::Op(resp.error.unwrap_or_default()))
            }
        }

        fn move_path(&mut self, node_id: u64, source: &str, destination: &str) -> HostResult<()> {
            let resp = host::nodefs::move_path(&nodefs::MoveRequest {
                node_id,
                source: source.to_owned(),
                destination: destination.to_owned(),
            })
            .map_err(call_err)?;
            if resp.success {
                Ok(())
            } else {
                Err(HostApiError::Op(resp.error.unwrap_or_default()))
            }
        }

        fn log_info(&mut self, message: &str) {
            host::log::info(message);
        }

        fn log_error(&mut self, message: &str) {
            host::log::error(message);
        }
    }
}
