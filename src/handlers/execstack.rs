//! POST /servers/{id}/platform/fix-execstack — clear the executable-stack flag
//! on a platform's native library.
//!
//! CounterStrikeSharp ships `counterstrikesharp.so` with `PT_GNU_STACK` marked
//! executable. Current kernels and glibc refuse that on `dlopen`, so Metamod
//! reports the plugin as `<ERROR>` and every `css_` console command disappears
//! — while the files on disk look perfect and reinstalling changes nothing,
//! because the release carries the same flag.
//!
//! The edit itself is four bytes. Getting them onto the server is the hard part.
//!
//! **Why this does not upload the patched library.** The obvious shape —
//! download, patch, upload — was built first and does not work: a single ~9.7MB
//! nodefs upload severs the daemon's gRPC session outright. The panel log shows
//! `daemon session unregistered` at the exact second of each attempt, a
//! reconnect a second later, and the write recorded as failed. Reads of the same
//! file are fine, so the ceiling is on the daemon's receive side, well below
//! both the panel's 10MB gRPC cap and this plugin's inline cap.
//!
//! So only the four changed bytes travel: they go up as a four-byte scratch file
//! and are written into place with `dd`, which is coreutils, needs no shell, and
//! is the same kind of node-side command the platform installer already runs.
//! `conv=notrunc` is load-bearing — without it `dd` truncates the library to
//! four bytes.
//!
//! Writing in place would be reckless against a library the running server has
//! mapped, but that is not the state this repairs: the flag is why `dlopen`
//! refused it, so nothing has it open.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::handlers::platform::require_exec_safe;
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult, json_response, parse_json_body};
use crate::model::{FixExecStackResponse, PlatformInstallRequest};
use crate::source2::{self, elf, paths};

/// Native libraries a platform loads, relative to the game dir.
fn library_rel(kind: &str) -> Option<&'static str> {
    match kind {
        "css" => Some(source2::CSS_LIBRARY),
        _ => None,
    }
}

const PATCH_FILE_NAME: &str = "execstack-patch.bin";

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    actor: Option<&str>,
) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let request: PlatformInstallRequest = parse_json_body(body)?;

    // Metamod's own library is loaded by the engine rather than dlopen'd the
    // same way and has never shown this failure; rather than guess at a path,
    // only the library known to need this is offered.
    let Some(rel) = library_rel(&request.kind) else {
        return Err(ApiError::unprocessable(
            "UNSUPPORTED_KIND",
            "only the CounterStrikeSharp library is known to ship with an executable stack",
        ));
    };

    let abs = paths::join(&ctx.game_abs, rel);
    if host.stat(ctx.node_id, &abs)?.is_none_or(|stat| stat.is_dir) {
        return Err(ApiError::not_found(
            "LIBRARY_NOT_FOUND",
            format!("{rel} is not on the server; install the platform first"),
        ));
    }

    // Reading the whole library is fine — it is the write direction the daemon
    // will not take. The program headers sit near the start, but nodefs has no
    // ranged read, so the whole file comes across.
    let bytes = host.download(ctx.node_id, &abs)?;
    let stack =
        elf::find_gnu_stack(&bytes).map_err(|err| ApiError::unprocessable("NOT_A_LIBRARY", err))?;

    let Some(stack) = stack.filter(|stack| stack.executable()) else {
        // Worth saying plainly: the caller came here because something was
        // wrong, and this rules a cause out rather than implying a repair.
        return Ok(json_response(
            200,
            &FixExecStackResponse {
                kind: request.kind,
                path: ctx.rel(rel),
                changed: false,
                before: None,
                after: None,
            },
        ));
    };

    let cleared = stack.flags & !elf::PF_X;
    write_flags(host, &ctx, &abs, stack.flags_offset, cleared)?;

    super::audit::record(
        host,
        ctx.server_id,
        actor,
        "platform-fix-execstack",
        &request.kind,
    );

    Ok(json_response(
        200,
        &FixExecStackResponse {
            kind: request.kind,
            path: ctx.rel(rel),
            changed: true,
            before: Some(stack.flags),
            after: Some(cleared),
        },
    ))
}

/// Puts four bytes at `offset` in a file on the node, and reads them back.
fn write_flags<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
    abs: &str,
    offset: usize,
    flags: u32,
) -> Result<(), ApiError> {
    require_exec_safe(abs)?;
    require_exec_safe(&ctx.root_abs)?;

    let scratch_abs = paths::join(&ctx.root_abs, source2::DOWNLOAD_SCRATCH_DIR);
    if host.stat(ctx.node_id, &scratch_abs)?.is_none() {
        host.mk_dir(ctx.node_id, &scratch_abs)?;
    }
    let patch_abs = paths::join(&scratch_abs, PATCH_FILE_NAME);
    host.upload(ctx.node_id, &patch_abs, &flags.to_le_bytes(), 0o644)?;

    let write = format!("dd if={patch_abs} of={abs} bs=1 seek={offset} count=4 conv=notrunc");
    let written = host.exec(ctx.node_id, &write, None);
    // The scratch file goes whatever happened; a stale one would be written
    // into the next library by a later run with a different offset.
    let _ = host.remove(ctx.node_id, &patch_abs, false);
    let written = written?;
    if written.exit_code != 0 {
        return Err(ApiError::unprocessable(
            "WRITE_FAILED",
            format!(
                "could not write to the library on the node (dd exited {}): {}",
                written.exit_code,
                written.output.trim()
            ),
        ));
    }

    // Read the bytes back rather than trusting an exit code: this edits a
    // library the server needs in order to start.
    let verify = format!("od -An -tx1 -j {offset} -N 4 {abs}");
    if let Ok(check) = host.exec(ctx.node_id, &verify, None)
        && check.exit_code == 0
        && let Some(seen) = parse_od_le_u32(&check.output)
        && seen != flags
    {
        return Err(ApiError::unprocessable(
            "VERIFY_FAILED",
            format!(
                "wrote {flags:#x} but the library reads back {seen:#x} (od said {:?})",
                check.output.trim()
            ),
        ));
    }
    Ok(())
}

/// Reads four bytes out of `od -An -tx1` output as a little-endian u32.
///
/// od prints them as two-digit lowercase hex, space-separated, in file order,
/// and `-An` suppresses everything else - so a well-formed answer is exactly
/// four such tokens and nothing more. Anything else returns `None` and the
/// check is skipped: od's formatting, or a node that decorates command output,
/// is not worth refusing a good write over.
///
/// The strictness is the point. A looser scan - any token that parses as hex,
/// take the first four - reads the numbers out of an echoed command line as if
/// they were file content, and `-N 4` alone is enough to shift the window and
/// report a perfectly good write as corrupt.
fn parse_od_le_u32(output: &str) -> Option<u32> {
    let mut bytes = [0u8; 4];
    let mut tokens = output.split_whitespace();
    for byte in &mut bytes {
        let token = tokens.next()?;
        if token.len() != 2 {
            return None;
        }
        *byte = u8::from_str_radix(token, 16).ok()?;
    }
    tokens.next().is_none().then(|| u32::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::parse_od_le_u32;

    #[test]
    fn reads_od_output() {
        assert_eq!(parse_od_le_u32(" 06 00 00 00\n"), Some(6));
        assert_eq!(parse_od_le_u32("07 00 00 00"), Some(7));
        assert_eq!(parse_od_le_u32("od: cannot open"), None);
        assert_eq!(parse_od_le_u32("06 00"), None);
        assert_eq!(parse_od_le_u32(""), None);
    }

    /// The regression this parser exists for: a node that echoes the command
    /// ahead of its output. The looser scan took the `4` out of `-N 4` as the
    /// first byte, read 0x604 where the library plainly said 6, and failed a
    /// write that had gone through perfectly.
    #[test]
    fn an_echoed_command_line_is_not_mistaken_for_file_content() {
        let echoed = "od -An -tx1 -j 516 -N 4 /srv/css.so\n 06 00 00 00\n";
        assert_eq!(parse_od_le_u32(echoed), None, "unreadable, never wrong");
    }
}
