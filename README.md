# kubeware

Container level HTTP Proxy for Kubernetes. Executes user defined middlewares before and after the request is sent to the original backend.

Useful when some functionality needs to be implemented outside of application or multiple runtimes are used and a shared libary is not feasible.

## How it works

RPC calls are done through GRPC

All middlewares, which are enabled for request stage are executed.  
Request headers can be added or removed, body altered.  
After that request is sent to the backend.  
After response from the backend is received, all middlewares for response stage are executed.  
Data from the request and the response is provided.  
Response headers can be added or removed, body altered, status code changed.  
After that the response is sent.  

Any middleware can stop the pipeline at any stage and define response headers, body and status code.  

Middlewares are executed in the order that they are defined in the config file.

## Use cases

- Authentication
- Authorization
- Tracing
- Auditing
- Cache-as-a-service
- Mirroring
- Data transformations

## Limitations

GRPC and WebSockets are not supported.

Currently only one set-cookie header will be sent. Investigation in #11

## Docker images

[kubeware](https://hub.docker.com/repository/docker/gedu17/kubeware)

[kubeware examples](https://hub.docker.com/repository/docker/gedu17/kubeware-examples)

## Examples

[Basic auth example](examples/authn/README.md)

[Mirroring example](examples/mirror/README.md)

[Data transformation example](examples/transform/README.md)

## Running locally

```cargo build```

```cargo run```

## Testing locally

```cargo test -- --test-threads 1```

## Configuration

Example with all possible values

```toml
ip = "127.0.0.1"
port = 17000
log = "info"

[backend]
url = "http://127.0.0.1:17001"
timeout_ms = 500
version = "HTTP"

[[middleware]]
url = "http://127.0.0.1:17002"
timeout_ms = 2000
request = true
response = false

[[middleware]]
url = "http://127.0.0.1:17003"
timeout_ms = 1500
request = false
response = true
```

### Kubeware configuration

`ip` - IP address to bind kubeware on. *Optional* - defaults to 127.0.0.1

`port` - Port number to bind kubeware on. *Optional* - defaults to 17000

`log` - Logging level. *Optional* - defaults to info. Possible values:

- Trace
- Debug
- Info
- Warn
- Error

### Backend configuration

`url` - HTTP endpoint for the backend. *Mandatory*

`timeout_ms` - Time to wait for the response from the backend. *Optional* - defaults to 5000 (5sec)

`version` - HTTP version to use. *Optional* - defaults to HTTP. Possible values: HTTP, HTTP2

### Middleware configuration

`url` - HTTP endpoint for the middleware. *Mandatory*

`timeout_ms` - Time to wait for the response from the middleware. *Optional* - defaults to 5000 (5sec)

`request` - Whether to send the `handle_request` RPC to the middleware or not. *Mandatory*

`response` - Whether to send the `handle_response` RPC to the middleware or not. *Mandatory*

### Environment variables

`CONFIG_FILE` - specify the location of the config file in the filesystem

`RUST_LOG` - specify log level


## Proto

### ResponseStatus

- `SUCCESS` - processing succeded, headers, body and status code (if applicable) are updated
- `CONTINUE` - processing pipeline proceeds, no data is updated
- `STOP` - processing failed, returns response to the requester with specified headers, body and status code

### Wrapper data types

Datatypes used from `wrappers.proto` are Optional (Nullable) types. 

For example if `HandleRequest` returns non-null body, then it will be updated, otherwise not. Same for status code.

## Status codes

`500` - Generic error - something went wrong inside kubeware

`502` - Connectivity issue to the backend

`503` - Connectivity issue to the middleware or middleware timed out

`504` - Backend timed out
