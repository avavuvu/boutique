use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
};
use std::convert::Infallible;
use tower::{Service, ServiceBuilder};
use tower_http::{services::{ServeDir, ServeFile}, set_header::SetResponseHeaderLayer};

use crate::{assets, middleware::base, models::user, state::AuthState, store::AuthUser};

pub struct Server<U: AuthUser = user::Model> {
    auth: AuthState<U>,
    static_dirs: Vec<(String, String)>,
    files: Vec<(String, String)>,
    debug: bool,
}

impl<U: AuthUser> Server<U> {
    pub fn new(auth: AuthState<U>) -> Self {
        Self {
            auth,
            static_dirs: Vec::new(),
            files: Vec::new(),
            debug: false,
        }
    }

    pub fn static_dir(mut self, route: impl Into<String>, dir: impl Into<String>) -> Self {
        self.static_dirs.push((route.into(), dir.into()));
        self
    }

    pub fn file(mut self, route: impl Into<String>, path: impl Into<String>) -> Self {
        self.files.push((route.into(), path.into()));
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
        let mut router = Router::new().merge(assets::router());

        for (route, path) in &self.files {
            router = router.route_service(route, ServeFile::new(path));
        }

        for (route, dir) in &self.static_dirs {
            let service = ServeDir::new(dir);
            router = if !self.debug {
                router.nest_service(route, service)
            } else {
                router.nest_service(
                    route,
                    ServiceBuilder::new()
                        .layer(SetResponseHeaderLayer::overriding(
                            header::CACHE_CONTROL,
                            HeaderValue::from_static("no-store"),
                        ))
                        .service(service),
                )
            };
        }

        let router = router
            .fallback_service(fallback)
            .layer(middleware::from_fn_with_state(self.auth, base));

        if self.debug {
            router
        } else {
            router.layer(middleware::from_fn(https_redirect))
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

async fn https_redirect(req: Request<Body>, next: Next) -> Response {
    let is_http = req
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "http")
        .unwrap_or(false);

    if is_http {
        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let path = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/");
        return Redirect::permanent(&format!("https://{host}{path}")).into_response();
    }

    next.run(req).await
}
