//! Scheduled maintenance: the gameinfo.gi auto-repair sweep. CS2 updates
//! silently strip the Metamod search path; this puts it back on every CS2
//! server that has Metamod installed, without anyone opening the panel.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::handlers::{audit, repair};
use crate::host_api::HostApi;
use crate::source2::{self, paths};

/// Never let one giant installation stall the scheduler slot.
const MAX_SERVERS_PER_SWEEP: usize = 100;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SweepStats {
    pub checked: u32,
    pub repaired: u32,
    pub failed: u32,
}

pub fn autorepair_sweep<H: HostApi>(host: &mut H) -> SweepStats {
    let mut stats = SweepStats::default();
    let servers = match host.find_servers_by_game(crate::GAME_CODE) {
        Ok(servers) => servers,
        Err(err) => {
            host.log_error(&format!("autorepair: server listing failed: {err:?}"));
            return stats;
        }
    };

    for server in servers.into_iter().take(MAX_SERVERS_PER_SWEEP) {
        let mut params = HashMap::new();
        params.insert("id".to_string(), server.id.to_string());
        let Ok(ctx) = ServerCtx::resolve(host, &params) else {
            continue; // not installed yet, wrong engine, node gone — skip
        };
        stats.checked += 1;

        // Only repair where Metamod is actually present; a bare server with
        // no addons dir is not broken, it is just not set up.
        let metamod_abs = paths::join(&ctx.game_abs, source2::METAMOD_DIR);
        let has_metamod = match host.stat(ctx.node_id, &metamod_abs) {
            Ok(stat) => stat.is_some_and(|s| s.is_dir),
            Err(_) => false,
        };
        if !has_metamod {
            continue;
        }

        match repair::repair(host, &ctx) {
            Ok(true) => {
                stats.repaired += 1;
                audit::record(
                    host,
                    ctx.server_id,
                    Some("auto-repair"),
                    "gameinfo-repair",
                    "gameinfo.gi",
                );
                host.log_info(&format!(
                    "autorepair: re-wired gameinfo.gi on server {}",
                    ctx.server_id
                ));
            }
            Ok(false) => {}
            Err(err) => {
                stats.failed += 1;
                host.log_error(&format!(
                    "autorepair: server {} failed: {}",
                    ctx.server_id, err.message
                ));
            }
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_api::mock::MockHost;

    #[test]
    fn sweep_repairs_only_unwired_servers_with_metamod() {
        let mut host = MockHost::cs2();
        // Server 3 (from cs2()) has metamod dir + unwired gameinfo.
        host.add_dir(&format!("{}/addons/metamod", MockHost::GAME_ABS));
        host.add_file(
            &format!("{}/gameinfo.gi", MockHost::GAME_ABS),
            b"SearchPaths\n{\n\tGame\tcsgo\n}\n",
        );

        let stats = autorepair_sweep(&mut host);
        assert_eq!(stats.checked, 1);
        assert_eq!(stats.repaired, 1);
        assert_eq!(stats.failed, 0);

        let patched = host
            .file(&format!("{}/gameinfo.gi", MockHost::GAME_ABS))
            .expect("gameinfo");
        assert!(crate::source2::gameinfo::is_metamod_wired(patched));

        // Second sweep is a no-op.
        let again = autorepair_sweep(&mut host);
        assert_eq!(again.repaired, 0);
    }

    #[test]
    fn sweep_skips_servers_without_metamod() {
        let mut host = MockHost::cs2();
        host.add_file(
            &format!("{}/gameinfo.gi", MockHost::GAME_ABS),
            b"SearchPaths\n{\n\tGame\tcsgo\n}\n",
        );
        let stats = autorepair_sweep(&mut host);
        assert_eq!(stats.checked, 1);
        assert_eq!(stats.repaired, 0);
    }
}
