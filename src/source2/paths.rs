//! Node path helpers. Node paths are plain strings joined with '/';
//! the daemon tolerates mixed separators on Windows nodes.

/// Joins an absolute base path with a relative part, collapsing duplicate '/'.
pub fn join(base: &str, rel: &str) -> String {
    let base = base.trim_end_matches('/');
    let rel = rel.trim_start_matches('/');
    if rel.is_empty() {
        return base.to_string();
    }
    if base.is_empty() {
        return rel.to_string();
    }
    format!("{base}/{rel}")
}

/// Returns the part after the last '/' or '\'.
pub fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Strips the last extension from a file name.
pub fn file_stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(0) | None => name,
        Some(idx) => &name[..idx],
    }
}

/// Replaces backslashes with forward slashes (for matching only,
/// never for rewriting file content).
pub fn normalize_slashes(path: &str) -> String {
    path.replace('\\', "/")
}

/// Validates a bare file name coming from a request.
///
/// nodefs operations run with the daemon's full privileges, so anything that
/// could escape the intended directory must be rejected.
pub fn sanitize_file_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("file name is empty");
    }
    if name.len() > 255 {
        return Err("file name is too long");
    }
    if name.contains(['/', '\\', '\0']) {
        return Err("file name must not contain path separators");
    }
    if name == "." || name == ".." {
        return Err("file name must not be a dot segment");
    }
    Ok(())
}

/// Validates a mod-dir-relative path coming from a request
/// (e.g. the metamod plugin location "addons/reunion/reunion_mm_i386.so").
pub fn sanitize_rel_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("path is empty");
    }
    if path.len() > 1024 {
        return Err("path is too long");
    }
    if path.contains(['\\', '\0']) {
        return Err("path must not contain backslashes");
    }
    if path.starts_with('/') {
        return Err("path must be relative");
    }
    for segment in path.split('/') {
        if segment.is_empty() {
            return Err("path must not contain empty segments");
        }
        if segment == "." || segment == ".." {
            return Err("path must not contain dot segments");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_paths() {
        assert_eq!(join("/srv/gameap", "servers/cs"), "/srv/gameap/servers/cs");
        assert_eq!(join("/srv/gameap/", "/servers/cs"), "/srv/gameap/servers/cs");
        assert_eq!(join("/srv", ""), "/srv");
        assert_eq!(join("C:\\gameap", "servers/cs"), "C:\\gameap/servers/cs");
    }

    #[test]
    fn file_name_and_stem() {
        assert_eq!(file_name("addons/reunion/reunion_mm_i386.so"), "reunion_mm_i386.so");
        assert_eq!(file_name("addons\\reunion\\reunion.dll"), "reunion.dll");
        assert_eq!(file_name("admin.amxx"), "admin.amxx");
        assert_eq!(file_stem("admin.amxx"), "admin");
        assert_eq!(file_stem("reunion_mm_i386.so"), "reunion_mm_i386");
        assert_eq!(file_stem("noext"), "noext");
        assert_eq!(file_stem(".hidden"), ".hidden");
    }

    #[test]
    fn sanitize_file_names() {
        assert!(sanitize_file_name("statsx.amxx").is_ok());
        assert!(sanitize_file_name("").is_err());
        assert!(sanitize_file_name("a/b.amxx").is_err());
        assert!(sanitize_file_name("a\\b.amxx").is_err());
        assert!(sanitize_file_name("..").is_err());
        assert!(sanitize_file_name(&"x".repeat(300)).is_err());
    }

    #[test]
    fn sanitize_rel_paths() {
        assert!(sanitize_rel_path("addons/reunion/reunion_mm_i386.so").is_ok());
        assert!(sanitize_rel_path("/addons/x.so").is_err());
        assert!(sanitize_rel_path("addons/../secret").is_err());
        assert!(sanitize_rel_path("addons//x.so").is_err());
        assert!(sanitize_rel_path("addons\\x.so").is_err());
    }
}
