# Data transformation example

This example updates the enum in the request body if the request is for the ```v2``` endpoint.

## Example

```curl <endpoint>/v1/endpoint --data '{"status": "ANYTHING"}' -H "Content-Type: application/json" --verbose```

This endpoint will return 201

```curl <endpoint>/v1/endpoint --data '{"status": "COMPLETED"}' -H "Content-Type: application/json" --verbose```

This endpoint will return 200 with id in json body

```curl <endpoint>/v2/endpoint --data '{"status": "ANYTHING"}' -H "Content-Type: application/json" --verbose```

For this endpoint ```ANYTHING``` will be translated to ```1``` and backend will return 201

```curl <endpoint>/v2/endpoint --data '{"status": "COMPLETED"}' -H "Content-Type: application/json" --verbose```

For this endpoint ```COMPLETED``` will be translated to ```0``` and backend will return 200 with id in json body

## Running in minikube

```kubectl apply -f kubernetes.yml```

```minikube service kubeware``` to get endpoint

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