//! Route table and dispatch. The matcher mirrors the host's `matchPath`
//! (segment-wise comparison, `{name}` captures, first match wins), so the
//! declared table stays the single source of truth.

use std::collections::HashMap;

use gameap_plugin_sdk::proto::gameap::plugin as pb;

use crate::handlers;
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RouteId {
    State,
    TogglePlugin,
    SetAttributes,
    AddPlugin,
    RemovePlugin,
}

pub struct RouteDef {
    pub id: RouteId,
    pub method: &'static str,
    pub pattern: &'static str,
    pub description: &'static str,
}

pub const ROUTES: &[RouteDef] = &[
    RouteDef {
        id: RouteId::State,
        method: "GET",
        pattern: "/servers/{id}/state",
        description: "Metamod:Source/CounterStrikeSharp state of a CS2 server",
    },
    RouteDef {
        id: RouteId::TogglePlugin,
        method: "POST",
        pattern: "/servers/{id}/plugins/toggle",
        description: "Enable or disable a plugin by moving its folder in/out of plugins/disabled",
    },
    RouteDef {
        id: RouteId::SetAttributes,
        method: "POST",
        pattern: "/servers/{id}/plugins/attributes",
        description: "Set a plugin's comment and group in plugins_meta.json",
    },
    RouteDef {
        id: RouteId::AddPlugin,
        method: "POST",
        pattern: "/servers/{id}/plugins",
        description: "Register an uploaded plugin folder in plugins_meta.json",
    },
    RouteDef {
        id: RouteId::RemovePlugin,
        method: "DELETE",
        pattern: "/servers/{id}/plugins",
        description: "Delete a plugin folder and its plugins_meta.json entry",
    },
];

pub fn http_routes() -> Vec<pb::HttpRoute> {
    ROUTES
        .iter()
        .map(|route| pb::HttpRoute {
            path: route.pattern.into(),
            methods: vec![route.method.into()],
            requires_auth: true,
            admin_only: true,
            description: route.description.into(),
        })
        .collect()
}

pub fn match_route(method: &str, path: &str) -> Option<(RouteId, HashMap<String, String>)> {
    ROUTES.iter().find_map(|route| {
        if !route.method.eq_ignore_ascii_case(method) {
            return None;
        }
        match_pattern(route.pattern, path).map(|params| (route.id, params))
    })
}

fn match_pattern(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    let pattern_segments: Vec<&str> = pattern.trim_matches('/').split('/').collect();
    let path_segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    if pattern_segments.len() != path_segments.len() {
        return None;
    }
    let mut params = HashMap::new();
    for (pat, seg) in pattern_segments.iter().zip(path_segments.iter()) {
        if let Some(name) = pat.strip_prefix('{').and_then(|p| p.strip_suffix('}')) {
            params.insert(name.to_string(), (*seg).to_string());
        } else if pat != seg {
            return None;
        }
    }
    Some(params)
}

pub fn dispatch<H: HostApi>(host: &mut H, req: &pb::HttpRequest) -> pb::HttpResponse {
    let Some((route, params)) = match_route(&req.method, &req.path) else {
        return ApiError::not_found("NOT_FOUND", "route not found").into_response();
    };
    let result: ApiResult = match route {
        RouteId::State => handlers::state::handle(host, &params),
        RouteId::TogglePlugin => handlers::toggle::handle(host, &params, &req.body),
        RouteId::SetAttributes => handlers::attributes::handle(host, &params, &req.body),
        RouteId::AddPlugin => handlers::add::handle(host, &params, &req.body),
        RouteId::RemovePlugin => {
            handlers::remove::handle(host, &params, &req.body, &req.query_params)
        }
    };
    result.unwrap_or_else(ApiError::into_response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_declared_routes() {
        let (id, params) = match_route("GET", "/servers/3/state").expect("route matches");
        assert_eq!(id, RouteId::State);
        assert_eq!(params.get("id").map(String::as_str), Some("3"));

        let (id, _) = match_route("post", "/servers/3/plugins/toggle").expect("route matches");
        assert_eq!(id, RouteId::TogglePlugin);

        let (id, _) = match_route("POST", "/servers/3/plugins/attributes").expect("route matches");
        assert_eq!(id, RouteId::SetAttributes);

        let (id, _) = match_route("POST", "/servers/3/plugins").expect("route matches");
        assert_eq!(id, RouteId::AddPlugin);

        let (id, _) = match_route("DELETE", "servers/3/plugins").expect("route matches");
        assert_eq!(id, RouteId::RemovePlugin);
    }

    #[test]
    fn rejects_unknown() {
        assert!(match_route("GET", "/servers/3/unknown").is_none());
        assert!(match_route("PUT", "/servers/3/state").is_none());
        assert!(match_route("GET", "/servers/3/state/extra").is_none());
        assert!(match_route("POST", "/servers/3/amxx/plugins/toggle").is_none());
        assert!(match_route("GET", "/").is_none());
    }
}
