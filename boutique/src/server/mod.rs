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
use tower_http::trace::TraceLayer;

pub use assets::ManifestError;
pub(crate) use assets::{HTMX_PATH, entry as manifest_entry};
pub use dev::run;
pub use files::Cache;
use files::{Mount, MountKind};
use layers::{compression, cross_origin_protection, https_redirect};

type Wrap = Box<dyn FnOnce(Router) -> Router + Send + Sync>;

#[must_use]
pub struct Server {
    build: Mount,
    mounts: Vec<Mount>,
    error_page: Option<Wrap>,
    debug: bool,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            build: Mount::new(assets::BUILD_ROUTE, assets::BUILD_DIR, Cache::Immutable, MountKind::Build),
            mounts: Vec::new(),
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
        self.build = Mount::new(route, dir, Cache::Immutable, MountKind::Build);
        self
    }

    pub fn static_dir(self, route: impl Into<String>, dir: impl Into<String>) -> Self {
        self.static_dir_with_cache(route, dir, Cache::Revalidate)
    }

    pub fn static_dir_with_cache(mut self, route: impl Into<String>, dir: impl Into<String>, cache: Cache) -> Self {
        self.mounts.push(Mount::new(route, dir, cache, MountKind::Dir));
        self
    }

    pub fn file(self, route: impl Into<String>, path: impl Into<String>) -> Self {
        self.file_with_cache(route, path, Cache::Revalidate)
    }

    pub fn file_with_cache(mut self, route: impl Into<String>, path: impl Into<String>, cache: Cache) -> Self {
        self.mounts.push(Mount::new(route, path, cache, MountKind::File));
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
        assets::init(&self.build.route, &self.build.path, self.debug)?;

        let router = self
            .mounts
            .iter()
            .chain([&self.build])
            .fold(Router::new().merge(assets::router()), |router, mount| {
                mount.add_to(router, self.debug)
            });

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
