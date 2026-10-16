mod assets;
mod dev;
mod error_page;
mod files;
mod layers;

use std::{convert::Infallible, io};

use axum::{
    Router,
    extract::Request,
    handler::Handler,
    middleware::{self, Next},
    response::Response,
};
use tokio::net::{TcpListener, ToSocketAddrs};
use tower::Service;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

pub use assets::ManifestError;
pub(crate) use assets::{HTMX_PATH, entry as manifest_entry};
pub use dev::run;
pub use files::Cache;
use files::{FilePolicy, files_service};
use layers::{compression, cross_origin_protection, https_redirect};

type Wrap = Box<dyn FnOnce(Router) -> Router + Send + Sync>;

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
                error_page::error_page(render.clone(), state.clone(), request, next)
            }))
        }));
        self
    }

    /// no-store on static files and no https redirect
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn router<F>(self, app: F) -> Result<Router, ManifestError>
    where
        F: Service<Request, Response = Response, Error = Infallible> + Clone + Send + Sync + 'static,
        F::Future: Send + 'static,
    {
        assets::init(&self.assets.0, &self.assets.1, self.debug)?;

        let mut router = Router::new().merge(assets::router());

        for (route, path, cache) in &self.files {
            let policy = FilePolicy::new(*cache, self.debug, false);
            router = router.route_service(route, files_service(policy, ServeFile::new(path)));
        }

        for (route, dir, cache) in &self.static_dirs {
            let policy = FilePolicy::new(*cache, self.debug, false);
            router = router.nest_service(route, files_service(policy, ServeDir::new(dir)));
        }

        let (route, dir) = &self.assets;
        let policy = FilePolicy::new(Cache::Immutable, self.debug, true);
        router = router.nest_service(route, files_service(policy, ServeDir::new(dir)));

        let router = router.fallback_service(app).layer(middleware::from_fn(cross_origin_protection));
        let router = match self.error_page {
            Some(wrap) => wrap(router),
            None => router,
        };
        let router = if self.debug {
            router
        } else {
            router.layer(middleware::from_fn(https_redirect))
        };

        Ok(router.layer(compression()).layer(TraceLayer::new_for_http()))
    }

    pub async fn serve<F>(self, app: F, address: impl ToSocketAddrs) -> io::Result<()>
    where
        F: Service<Request, Response = Response, Error = Infallible> + Clone + Send + Sync + 'static,
        F::Future: Send + 'static,
    {
        let router = self.router(app).map_err(io::Error::other)?;
        let listener = TcpListener::bind(address).await?;
        axum::serve(listener, router.into_make_service()).await
    }
}
