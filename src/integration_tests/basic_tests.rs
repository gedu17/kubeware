#[cfg(test)]
mod tests {
    use tonic::{Request as TonicRequest, Response as TonicResponse};
    use crate::kubeware::{RequestRequest, RequestResponse, ResponseRequest, ResponseResponse, ResponseStatus};
    use crate::integration_tests::{setup_middleware, setup_kubeware, setup_backend, BackendFn};
    use hyper::{Body, Client, Request, Response};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use crate::{KUBEWARE_TIME_HEADER, BACKEND_TIME_HEADER};

    const CONFIG: &str = r#"
        ip = "127.0.0.1"
        port = 17000

        [backend]
        url = "http://127.0.0.1:17001"
        version = "HTTP"

        [[services]]
        url = "http://127.0.0.1:17002"
        request = true
        response = true
    "#;

    #[tokio::test(core_threads = 5)]
    async fn basic_test() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Arrange
        let request_counter = Arc::new(AtomicUsize::new(0));
        let response_counter = Arc::new(AtomicUsize::new(0));
        let request_counter_closure = request_counter.clone();
        let response_counter_closure = response_counter.clone();

        let middleware_tx = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                request_counter_closure.store(1, Ordering::Relaxed);
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                response_counter_closure.store(1, Ordering::Relaxed);
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;
        
        let kubeware_tx = setup_kubeware(CONFIG).await?;
        let backend_tx = setup_backend(|_req| {
            Response::new(Body::from(format!("OK")))
        }).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .body(Body::empty())
            .unwrap();

        let res = Client::new().request(req).await?;
        let (parts, body) = res.into_parts();

        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(1, response_counter.load(Ordering::Relaxed));
        assert_eq!(200, parts.status.as_u16());
        assert_eq!("OK", hyper::body::to_bytes(body).await?);
        assert!(parts.headers.get(KUBEWARE_TIME_HEADER).is_some());
        assert!(parts.headers.get(BACKEND_TIME_HEADER).is_some());

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }
}