use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    handler::Handler,
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
};
use percent_encoding::percent_decode_str;
use std::{convert::Infallible, time::Duration};
use tower::{Service, ServiceBuilder};
use tower_http::{
    compression::{
        CompressionLayer, Predicate,
        predicate::{DefaultPredicate, NotForContentType},
    },
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{
    assets,
    middleware::{cross_origin_protection, error_page},
};

type Wrap = Box<dyn FnOnce(Router) -> Router + Send + Sync>;

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

#[derive(Clone)]
struct FilePolicy {
    cache_control: HeaderValue,
    debug: bool,
    hide_dotfiles: bool,
}

pub struct Server {
    assets: (String, String),
    static_dirs: Vec<(String, String, Cache)>,
    files: Vec<(String, String, Cache)>,
    error_page: Option<Wrap>,
    debug: bool,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            assets: (assets::BUILD_ROUTE.into(), assets::BUILD_DIR.into()),
            static_dirs: Vec::new(),
            files: Vec::new(),
            error_page: None,
            debug: false,
        }
    }
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assets(mut self, route: impl Into<String>, dir: impl Into<String>) -> Self {
        self.assets = (route.into(), dir.into());
        self
    }

    pub fn static_dir(self, route: impl Into<String>, dir: impl Into<String>) -> Self {
        self.static_dir_with(route, dir, Cache::Revalidate)
    }

    pub fn static_dir_with(mut self, route: impl Into<String>, dir: impl Into<String>, cache: Cache) -> Self {
        self.static_dirs.push((route.into(), dir.into(), cache));
        self
    }

    pub fn file(self, route: impl Into<String>, path: impl Into<String>) -> Self {
        self.file_with(route, path, Cache::Revalidate)
    }

    pub fn file_with(mut self, route: impl Into<String>, path: impl Into<String>, cache: Cache) -> Self {
        self.files.push((route.into(), path.into(), cache));
        self
    }

    pub fn error_page<H, T>(self, render: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.error_page_with_state((), render)
    }

    pub fn error_page_with_state<H, T, S>(mut self, state: S, render: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
        S: Clone + Send + Sync + 'static,
    {
        self.error_page = Some(Box::new(move |router: Router| {
            router.layer(middleware::from_fn(move |request: Request, next: Next| {
                error_page(render.clone(), state.clone(), request, next)
            }))
        }));
        self
    }

    /// no-store on static files and no https redirect
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn router<F>(self, fallback: F) -> Router
    where
        F: Service<Request, Response = Response, Error = Infallible> + Clone + Send + Sync + 'static,
        F::Future: Send + 'static,
    {
        assets::manifest::init(&self.assets.0, &self.assets.1);

        let mut router = Router::new().merge(assets::router());

        for (route, path, cache) in &self.files {
            let policy = self.file_policy(*cache, false);
            router = router.route_service(route, files_service(policy, ServeFile::new(path)));
        }

        for (route, dir, cache) in &self.static_dirs {
            let policy = self.file_policy(*cache, false);
            router = router.nest_service(route, files_service(policy, ServeDir::new(dir)));
        }

        let (route, dir) = &self.assets;
        let policy = self.file_policy(Cache::Immutable, true);
        router = router.nest_service(route, files_service(policy, ServeDir::new(dir)));

        let router = router
            .fallback_service(fallback)
            .layer(middleware::from_fn(cross_origin_protection));
        let router = match self.error_page {
            Some(wrap) => wrap(router),
            None => router,
        };

        let router = if self.debug {
            router
        } else {
            router.layer(middleware::from_fn(https_redirect))
        };

        router.layer(compression()).layer(TraceLayer::new_for_http())
    }

    fn file_policy(&self, cache: Cache, hide_dotfiles: bool) -> FilePolicy {
        let cache_control = if self.debug {
            HeaderValue::from_static("no-store")
        } else {
            cache.header_value()
        };
        FilePolicy {
            cache_control,
            debug: self.debug,
            hide_dotfiles,
        }
    }

    pub async fn serve<F>(self, fallback: F, port: &str)
    where
        F: Service<Request, Response = Response, Error = Infallible> + Clone + Send + Sync + 'static,
        F::Future: Send + 'static,
    {
        let router = self.router(fallback);
        let address = format!("0.0.0.0:{port}");
        let listener = tokio::net::TcpListener::bind(&address).await.expect("failed to bind");
        axum::serve(listener, router.into_make_service()).await.expect("server error");
    }
}

fn compression() -> CompressionLayer<impl Predicate> {
    CompressionLayer::new().compress_when(
        DefaultPredicate::new()
            .and(NotForContentType::const_new("video/"))
            .and(NotForContentType::const_new("audio/"))
            .and(NotForContentType::const_new("font/woff")),
    )
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

async fn https_redirect(req: Request<Body>, next: Next) -> Response {
    let is_http = req
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "http")
        .unwrap_or(false);

    if is_http {
        let host = req.headers().get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("");
        let path = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/");
        return Redirect::permanent(&format!("https://{host}{path}")).into_response();
    }

    next.run(req).await
}
