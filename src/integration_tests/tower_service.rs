// use hyper::{Request, Body, Response};
// use hyper::service::Service;
// use std::task::{Context, Poll};
// use futures::future;
//
// use crate::integration_tests::BackendFn;
// use std::pin::Pin;
// use std::future::Future;
//
// pub struct TestBuilder
// {
//     pub backend_fn: BackendFn
// }
//
// pub struct TestRequestHandler
// {
//     pub backend_fn: &'static BackendFn
// }
//
// impl<T> Service<T> for TestBuilder {
//     type Response = TestRequestHandler;
//     type Error = std::io::Error;
//     type Future = future::Ready<Result<Self::Response, Self::Error>>;
//
//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Ok(()).into()
//     }
//
//     fn call(&mut self, _: T) -> Self::Future {
//         future::ok(TestRequestHandler {
//             backend_fn: &self.backend_fn
//         })
//     }
// }
//
//
//
// impl Service<Request<Body>> for TestRequestHandler {
//     type Response = Response<Body>;
//     type Error = hyper::Error;
//     type Future = Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Unpin + Send>;//(dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static);
//
//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Ok(()).into()
//     }
//
//     fn call(&mut self, req: Request<Body>) -> Self::Future {
//         // let services = Arc::clone(&self.services);
//         // let config = self.config.clone();
//         // let http_client = self.http_client.clone();
//         //
//         Box::new(future::ok((self.backend_fn)(req)))
//
//         // let executor = async move {
//         //     Ok((self.backend_fn)(req))
//         //     match RequestHandler::handle(req, services, config, http_client).await {
//         //         Ok(val) => Ok(val),
//         //         Err(err) => {
//         //             error!("Failed to parse request: {:?}", err);
//         //
//         //             Ok(RequestHandler::generic_error())
//         //         }
//         //     }
//         // };
//         //
//         // Box::pin(executor)
//         // Box::pin((self.backend_fn)(req))
//     }
// }