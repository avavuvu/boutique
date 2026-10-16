use std::{convert::Infallible, time::Duration};

use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use percent_encoding::percent_decode_str;
use tower::{Service, ServiceBuilder};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cache {
    #[default]
    Revalidate,
    Immutable,
    MaxAge(Duration),
}

impl Cache {
    fn header_value(self) -> HeaderValue {
        match self {
            Cache::Revalidate => HeaderValue::from_static("no-cache"),
            Cache::Immutable => HeaderValue::from_static("public, max-age=31536000, immutable"),
            Cache::MaxAge(age) => HeaderValue::try_from(format!("public, max-age={}", age.as_secs())).expect("a valid cache-control value"),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum MountKind {
    File,
    Dir,
    Build,
}

pub(super) struct Mount {
    pub(super) route: String,
    pub(super) path: String,
    cache: Cache,
    kind: MountKind,
}

impl Mount {
    pub(super) fn new(route: impl Into<String>, path: impl Into<String>, cache: Cache, kind: MountKind) -> Self {
        Self {
            route: route.into(),
            path: path.into(),
            cache,
            kind,
        }
    }

    pub(super) fn add_to(&self, router: Router, debug: bool) -> Router {
        let policy = FilePolicy::new(self.cache, debug, matches!(self.kind, MountKind::Build));
        match self.kind {
            MountKind::File => router.route_service(&self.route, files_service(policy, ServeFile::new(&self.path))),
            MountKind::Dir | MountKind::Build => router.nest_service(&self.route, files_service(policy, ServeDir::new(&self.path))),
        }
    }
}

#[derive(Clone)]
struct FilePolicy {
    cache_control: HeaderValue,
    debug: bool,
    hide_dotfiles: bool,
}

impl FilePolicy {
    fn new(cache: Cache, debug: bool, hide_dotfiles: bool) -> Self {
        let cache_control = if debug {
            HeaderValue::from_static("no-store")
        } else {
            cache.header_value()
        };
        FilePolicy {
            cache_control,
            debug,
            hide_dotfiles,
        }
    }
}

fn files_service<S>(
    policy: FilePolicy,
    service: S,
) -> impl Service<Request, Response = Response, Error = Infallible, Future: Send> + Clone + Send + Sync + 'static
where
    S: Service<Request, Error = Infallible> + Clone + Send + Sync + 'static,
    S::Response: IntoResponse,
    S::Future: Send + 'static,
{
    ServiceBuilder::new()
        .layer(middleware::from_fn_with_state(policy, file_headers))
        .service(service)
}

async fn file_headers(State(policy): State<FilePolicy>, request: Request, next: Next) -> Response {
    if policy.hide_dotfiles && has_dotfile(request.uri().path()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut response = next.run(request).await;
    let status = response.status();
    if policy.debug || status.is_success() || status == StatusCode::NOT_MODIFIED {
        response.headers_mut().insert(header::CACHE_CONTROL, policy.cache_control);
    }
    response
}

fn has_dotfile(path: &str) -> bool {
    let decoded = percent_decode_str(path).decode_utf8_lossy();
    decoded.split(['/', '\\']).any(|segment| segment.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::has_dotfile;

    #[test]
    fn dotfiles() {
        let cases = [
            ("/", false),
            ("/app-1234.js", false),
            ("/assets/app.min.css", false),
            ("/assets/..app.css", true),
            ("/.vite/manifest.json", true),
            ("/assets/.hidden", true),
            ("/assets/.hidden/app.js", true),
            ("/%2evite/manifest.json", true),
            ("/%2Evite/manifest.json", true),
            ("/assets%2f.vite/manifest.json", true),
            ("/assets/%5c.vite", true),
            ("/assets\\.vite", true),
            ("/../secret", true),
        ];

        for (path, expected) in cases {
            assert_eq!(has_dotfile(path), expected, "{path}");
        }
    }
}
