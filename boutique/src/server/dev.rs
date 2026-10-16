use std::future::Future;

/// runs the server. with the `devtools` feature the callback is re-run after
/// each `dx serve --hot-patch` patch instead of once.
pub async fn run<A, F>(args: A, callback: impl FnMut(A) -> F)
where
    A: Clone,
    F: Future<Output = ()> + 'static,
{
    #[cfg(feature = "devtools")]
    dioxus_devtools::serve_subsecond_with_args(args, callback).await;

    #[cfg(not(feature = "devtools"))]
    {
        let mut callback = callback;
        callback(args).await;
    }
}
