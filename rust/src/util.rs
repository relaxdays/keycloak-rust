/// wrapper trait to specify the correct lifetime bounds for the callback passed to [`Keycloak::with_client`](crate::Keycloak::with_client)
///
/// due to the way `async fn`s are desugared, a trait like this is necessary. the blanket impl of this trait for
/// `FnOnce(&'a Client) -> impl Future<...> + 'a` allows `async fn`s to be passed directly to `with_client`.
pub trait WithClientAsyncFn<'a, Res> {
    fn call(
        self,
        client: &'a crate::rest::Client,
    ) -> impl std::future::Future<Output = Result<Res, crate::Error>> + Send + 'a;
}

impl<'a, Res, Fut, F> WithClientAsyncFn<'a, Res> for F
where
    F: FnOnce(&'a crate::rest::Client) -> Fut,
    Fut: std::future::Future<Output = Result<Res, crate::Error>> + Send + 'a,
{
    fn call(
        self,
        client: &'a crate::rest::Client,
    ) -> impl std::future::Future<Output = Result<Res, crate::Error>> + Send + 'a {
        self(client)
    }
}
