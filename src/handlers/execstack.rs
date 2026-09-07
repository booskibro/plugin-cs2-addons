//! POST /servers/{id}/platform/fix-execstack — clear the executable-stack flag
//! on a platform's native library.
//!
//! CounterStrikeSharp ships `counterstrikesharp.so` with `PT_GNU_STACK` marked
//! executable. Current kernels and glibc refuse that on `dlopen`, so Metamod
//! reports the plugin as `<ERROR>` and every `css_` console command disappears
//! — while the files on disk look perfect and reinstalling changes nothing,
//! because the release carries the same flag.
//!
//! The repair is four bytes: clear `PF_X` on that one program header, which is
//! what `execstack -c` does. It happens here rather than through a node-side
//! command because `execstack` lived in `prelink`, dropped by recent
//! distributions — so the documented fix is missing on exactly the systems new
//! enough to need it.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
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

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    actor: Option<&str>,
) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let request: PlatformInstallRequest = parse_json_body(body)?;

    // Metamod's own library is loaded by the engine rather than dlopen'd the
    // same way, and has never shown this failure; rather than guess at its
    // path, only what is known to need it is offered.
    let Some(rel) = library_rel(&request.kind) else {
        return Err(ApiError::unprocessable(
            "UNSUPPORTED_KIND",
            "only the css library is known to ship with an executable stack",
        ));
    };

    let abs = paths::join(&ctx.game_abs, rel);
    if host.stat(ctx.node_id, &abs)?.is_none_or(|stat| stat.is_dir) {
        return Err(ApiError::not_found(
            "LIBRARY_NOT_FOUND",
            format!("{rel} is not on the server; install the platform first"),
        ));
    }

    let mut bytes = host.download(ctx.node_id, &abs)?;
    let patched = elf::clear_exec_stack(&mut bytes)
        .map_err(|err| ApiError::unprocessable("NOT_A_LIBRARY", err))?;

    let Some((before, after)) = patched else {
        // Worth saying plainly: the caller came here because something was
        // wrong, and this rules out one cause instead of implying a repair.
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

    // 0o755: the library must stay executable, and upload sets the mode.
    host.upload(ctx.node_id, &abs, &bytes, 0o755)?;

    super::audit::record(host, ctx.server_id, actor, "platform-fix-execstack", &request.kind);

    Ok(json_response(
        200,
        &FixExecStackResponse {
            kind: request.kind,
            path: ctx.rel(rel),
            changed: true,
            before: Some(before),
            after: Some(after),
        },
    ))
}
