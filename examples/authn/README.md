# Authorization header example

This example checks for authorization header, and verifies these credentials ```admin``` / ```password123```.

For request to the backend it removes authorization header and replaces it with ```user: admin``` header.


## Running in minikube

```kubectl apply -f kubernetes.yml```

```minikube service kubeware```

## Running locally

```npm install```

```node server.js```

```node middleware.js```

Then set service configuration:

```
[backend]
url = "http://127.0.0.1:17001"

[[middleware]]
url = "http://127.0.0.1:17002"
request = true
response = false
```