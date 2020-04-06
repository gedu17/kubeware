# Data transformation example

This example updates the enum in the request body if the request is for the ```v2``` endpoint.

To run this example:

```npm install```

```node server.js```

```node middleware.js```

Then set service configuration:

```
[backend]
url = "http://127.0.0.1:17001"

[[services]]
url = "http://127.0.0.1:17002"
request = true
response = false
```