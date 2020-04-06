# Authorization header example

This example checks for authorization header, and verifies these credentials ```admin``` / ```password123```. Also it removes authorization header and replaces it with ```user: admin``` header.

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