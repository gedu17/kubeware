# Request mirroring example

This example sends mirrored request to mirror endpoint

## Running in minikube

```kubectl apply -f kubernetes.yml```

```minikube service kubeware```

```secondservice``` container will receive all requests sent to ```httpbin``` container

## Running locally

```npm install```

```node server.js```

```node mirror.js```

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