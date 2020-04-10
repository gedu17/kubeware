#[cfg(test)]
mod tests {
    use tonic::{Request as TonicRequest, Response as TonicResponse};
    use crate::kubeware::{RequestRequest, RequestResponse, ResponseRequest, ResponseResponse, ResponseStatus, Header};
    use crate::integration_tests::{setup_middleware, setup_kubeware, setup_backend, BackendResponse, setup_backend2};
    use hyper::{Body, Client, Request, Response, HeaderMap};
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{Ordering};
    use async_trait::async_trait;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

    const HEADER_NAME: &str = "x-test-header";
    const HEADER2_NAME: &str = "x-test-header2";
    const CONFIG: &str = r#"
        ip = "127.0.0.1"
        port = 17000

        [backend]
        url = "http://127.0.0.1:17001"
        version = "HTTP"

        [[middleware]]
        url = "http://127.0.0.1:17002"
        request = true
        response = false
    "#;

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_request_continue_full_flow_is_executed() -> Result<()> {
        // Arrange
        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Continue as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;
        let kubeware_tx = setup_kubeware(CONFIG).await?;

        let (backend_tx, backend_counter) = setup_backend(move |_req| {
            Response::new(Body::from(format!("OK")))
        }).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .body(Body::empty())
            .unwrap();

        let res = Client::new().request(req).await?;
        let (parts, body) = res.into_parts();

        // Assert
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(1, backend_counter.load(Ordering::Relaxed));
        assert_eq!(200, parts.status.as_u16());
        assert_eq!("OK", hyper::body::to_bytes(body).await?);

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_continue_and_body_change_body_is_not_changed() -> Result<()> {
        // Arrange
        #[derive(Clone)]
        pub struct Backend {
            pub body: Arc<Mutex<String>>
        }

        #[async_trait]
        impl BackendResponse for Backend {
            async fn handle(&mut self, request: Request<Body>) -> Response<Body> {
                let (_parts, body) = request.into_parts();
                let full_body = hyper::body::to_bytes(body).await.unwrap();
                *self.body.lock().unwrap() = std::str::from_utf8(full_body.as_ref()).unwrap().to_string();

                Response::new(Body::from(format!("OK")))
            }
        }

        let changed_body = "New body !";
        let original_body = "Real body !";

        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Continue as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: Some(changed_body.to_string()),
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;

        let kubeware_tx = setup_kubeware(CONFIG).await?;
        let backend = Backend { body: Arc::new(Mutex::new(String::default())) };
        let backend_body = Arc::clone(&backend.body);
        let (backend_tx, backend_counter) = setup_backend2(backend).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .method("POST")
            .body(Body::from(original_body))
            .unwrap();

        let _ = Client::new().request(req).await?;

        // Assert
        assert_eq!(original_body, *backend_body.lock().unwrap());
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(1, backend_counter.load(Ordering::Relaxed));

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_continue_and_headers_added_headers_are_not_added() -> Result<()> {
        // Arrange
        #[derive(Clone)]
        pub struct Backend {
            pub headers: Arc<Mutex<HeaderMap>>
        }

        #[async_trait]
        impl BackendResponse for Backend {
            async fn handle(&mut self, request: Request<Body>) -> Response<Body> {
                let (parts, _body) = request.into_parts();
                self.headers.lock().unwrap().extend(parts.headers);

                Response::new(Body::from(format!("OK")))
            }
        }

        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Continue as i32,
                    added_headers: vec![
                        Header {
                            name: HEADER_NAME.to_string(),
                            value: 1.to_string()
                        },
                        Header {
                            name: HEADER2_NAME.to_string(),
                            value: 1.to_string()
                        },
                    ],
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;

        let kubeware_tx = setup_kubeware(CONFIG).await?;
        let backend = Backend { headers: Arc::new(Mutex::new(HeaderMap::new())) };
        let backend_headers = Arc::clone(&backend.headers);
        let (backend_tx, backend_counter) = setup_backend2(backend).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .method("POST")
            .body(Body::from("test body"))
            .unwrap();

        let _ = Client::new().request(req).await?;
        let headers = backend_headers.lock().unwrap();

        // Assert
        assert!(!headers.contains_key(HEADER_NAME));
        assert!(!headers.contains_key(HEADER2_NAME));
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(1, backend_counter.load(Ordering::Relaxed));

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_continue_and_headers_removed_headers_are_not_removed() -> Result<()> {
        // Arrange
        #[derive(Clone)]
        pub struct Backend {
            pub headers: Arc<Mutex<HeaderMap>>
        }

        #[async_trait]
        impl BackendResponse for Backend {
            async fn handle(&mut self, request: Request<Body>) -> Response<Body> {
                let (parts, _body) = request.into_parts();
                self.headers.lock().unwrap().extend(parts.headers);

                Response::new(Body::from(format!("OK")))
            }
        }

        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Continue as i32,
                    added_headers:  Vec::default(),
                    removed_headers: vec![HEADER_NAME.to_string(), HEADER2_NAME.to_string()],
                    body: None,
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;

        let kubeware_tx = setup_kubeware(CONFIG).await?;
        let backend = Backend { headers: Arc::new(Mutex::new(HeaderMap::new())) };
        let backend_headers = Arc::clone(&backend.headers);
        let (backend_tx, backend_counter) = setup_backend2(backend).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .method("POST")
            .header(HEADER_NAME, "1")
            .header(HEADER2_NAME, "1")
            .body(Body::from("test body"))
            .unwrap();

        let _ = Client::new().request(req).await?;
        let headers = backend_headers.lock().unwrap();

        // Assert
        assert!(headers.contains_key(HEADER_NAME));
        assert!(headers.contains_key(HEADER2_NAME));
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(1, backend_counter.load(Ordering::Relaxed));

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_request_success_full_flow_is_executed() -> Result<()> {
        // Arrange
        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;
        let kubeware_tx = setup_kubeware(CONFIG).await?;

        let (backend_tx, backend_counter) = setup_backend(move |_req| {
            Response::new(Body::from(format!("OK")))
        }).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .body(Body::empty())
            .unwrap();

        let res = Client::new().request(req).await?;
        let (parts, body) = res.into_parts();

        // Assert
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(1, backend_counter.load(Ordering::Relaxed));
        assert_eq!(200, parts.status.as_u16());
        assert_eq!("OK", hyper::body::to_bytes(body).await?);

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_request_stop_without_status_code_flow_is_stopped_and_500_returned() -> Result<()> {
        // Arrange
        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Stop as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;
        let kubeware_tx = setup_kubeware(CONFIG).await?;

        let (backend_tx, backend_counter) = setup_backend(move |_req| {
            Response::new(Body::from(format!("OK")))
        }).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .body(Body::empty())
            .unwrap();

        let res = Client::new().request(req).await?;
        let (parts, body) = res.into_parts();

        // Assert
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(0, backend_counter.load(Ordering::Relaxed));
        assert_eq!(500 as u16, parts.status.as_u16());
        assert_eq!("", hyper::body::to_bytes(body).await?);

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_request_stop_flow_is_stopped_with_custom_body_and_status_code() -> Result<()> {
        // Arrange
        let response_body = "Whoops :(";
        let status_code = 404;

        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Stop as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: Some(response_body.to_string()),
                    status_code: Some(status_code)
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;
        let kubeware_tx = setup_kubeware(CONFIG).await?;

        let (backend_tx, backend_counter) = setup_backend(move |_req| {
            Response::new(Body::from(format!("OK")))
        }).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .body(Body::empty())
            .unwrap();

        let res = Client::new().request(req).await?;
        let (parts, body) = res.into_parts();

        // Assert
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(0, backend_counter.load(Ordering::Relaxed));
        assert_eq!(status_code as u16, parts.status.as_u16());
        assert_eq!(response_body, hyper::body::to_bytes(body).await?);

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_request_stop_flow_is_stopped_with_added_headers() -> Result<()> {
        // Arrange
        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Stop as i32,
                    added_headers: vec![
                        Header {
                            name: HEADER_NAME.to_string(),
                            value: 1.to_string()
                        },
                        Header {
                            name: HEADER2_NAME.to_string(),
                            value: 2.to_string()
                        }
                    ],
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;
        let kubeware_tx = setup_kubeware(CONFIG).await?;

        let (backend_tx, backend_counter) = setup_backend(move |_req| {
            Response::new(Body::from(format!("OK")))
        }).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .body(Body::empty())
            .unwrap();

        let res = Client::new().request(req).await?;
        let (parts, body) = res.into_parts();

        // Assert
        assert!(parts.headers.contains_key(HEADER_NAME));
        assert!(parts.headers.contains_key(HEADER2_NAME));
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(0, backend_counter.load(Ordering::Relaxed));
        assert_eq!(500 as u16, parts.status.as_u16());
        assert_eq!("", hyper::body::to_bytes(body).await?);

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_success_and_body_change_body_is_changed() -> Result<()> {
        // Arrange
        #[derive(Clone)]
        pub struct Backend {
            pub body: Arc<Mutex<String>>
        }

        #[async_trait]
        impl BackendResponse for Backend {
            async fn handle(&mut self, request: Request<Body>) -> Response<Body> {
                let (_parts, body) = request.into_parts();
                let full_body = hyper::body::to_bytes(body).await.unwrap();
                *self.body.lock().unwrap() = std::str::from_utf8(full_body.as_ref()).unwrap().to_string();

                Response::new(Body::from(format!("OK")))
            }
        }

        let changed_body = "New body !";

        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: Some(changed_body.to_string()),
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;

        let kubeware_tx = setup_kubeware(CONFIG).await?;
        let backend = Backend { body: Arc::new(Mutex::new(String::default())) };
        let backend_body = Arc::clone(&backend.body);
        let (backend_tx, backend_counter) = setup_backend2(backend).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .method("POST")
            .body(Body::from("test body"))
            .unwrap();

        let _ = Client::new().request(req).await?;

        // Assert
        assert_eq!(changed_body, *backend_body.lock().unwrap());
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(1, backend_counter.load(Ordering::Relaxed));

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_success_and_headers_added_headers_are_added() -> Result<()> {
        // Arrange
        #[derive(Clone)]
        pub struct Backend {
            pub headers: Arc<Mutex<HeaderMap>>
        }

        #[async_trait]
        impl BackendResponse for Backend {
            async fn handle(&mut self, request: Request<Body>) -> Response<Body> {
                let (parts, _body) = request.into_parts();
                self.headers.lock().unwrap().extend(parts.headers);

                Response::new(Body::from(format!("OK")))
            }
        }

        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: vec![
                        Header {
                            name: HEADER_NAME.to_string(),
                            value: 1.to_string()
                        },
                        Header {
                            name: HEADER2_NAME.to_string(),
                            value: 1.to_string()
                        },
                    ],
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;

        let kubeware_tx = setup_kubeware(CONFIG).await?;
        let backend = Backend { headers: Arc::new(Mutex::new(HeaderMap::new())) };
        let backend_headers = Arc::clone(&backend.headers);
        let (backend_tx, backend_counter) = setup_backend2(backend).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .method("POST")
            .body(Body::from("test body"))
            .unwrap();

        let _ = Client::new().request(req).await?;
        let headers = backend_headers.lock().unwrap();

        // Assert
        assert!(headers.contains_key(HEADER_NAME));
        assert!(headers.contains_key(HEADER2_NAME));
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(1, backend_counter.load(Ordering::Relaxed));

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }

    #[tokio::test(core_threads = 5)]
    async fn when_sending_request_on_success_and_headers_removed_headers_are_removed() -> Result<()> {
        // Arrange
        #[derive(Clone)]
        pub struct Backend {
            pub headers: Arc<Mutex<HeaderMap>>
        }

        #[async_trait]
        impl BackendResponse for Backend {
            async fn handle(&mut self, request: Request<Body>) -> Response<Body> {
                let (parts, _body) = request.into_parts();
                self.headers.lock().unwrap().extend(parts.headers);

                Response::new(Body::from(format!("OK")))
            }
        }

        let (middleware_tx, request_counter, response_counter) = setup_middleware(
            Box::new(move |_req: TonicRequest<RequestRequest>| {
                TonicResponse::new(RequestResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers:  Vec::default(),
                    removed_headers: vec![HEADER_NAME.to_string(), HEADER2_NAME.to_string()],
                    body: None,
                    status_code: None
                })
            }),
            Box::new(move |_req: TonicRequest<ResponseRequest>| {
                TonicResponse::new(ResponseResponse {
                    status: ResponseStatus::Success as i32,
                    added_headers: Vec::default(),
                    removed_headers: Vec::default(),
                    body: None,
                    status_code: None
                })
            })).await?;

        let kubeware_tx = setup_kubeware(CONFIG).await?;
        let backend = Backend { headers: Arc::new(Mutex::new(HeaderMap::new())) };
        let backend_headers = Arc::clone(&backend.headers);
        let (backend_tx, backend_counter) = setup_backend2(backend).await?;

        // Act
        let req = Request::builder()
            .uri("http://127.0.0.1:17000/")
            .method("POST")
            .header(HEADER_NAME, "1")
            .header(HEADER2_NAME, "1")
            .body(Body::from("test body"))
            .unwrap();

        let _ = Client::new().request(req).await?;
        let headers = backend_headers.lock().unwrap();

        // Assert
        assert!(!headers.contains_key(HEADER_NAME));
        assert!(!headers.contains_key(HEADER2_NAME));
        assert_eq!(1, request_counter.load(Ordering::Relaxed));
        assert_eq!(0, response_counter.load(Ordering::Relaxed));
        assert_eq!(1, backend_counter.load(Ordering::Relaxed));

        // Cleanup
        let _ = kubeware_tx.send(());
        let _ = backend_tx.send(());
        let _ = middleware_tx.send(());

        Ok(())
    }
}