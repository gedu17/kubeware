# Request mirroring example

This example sends mirrored request to mirror endpoint

To run this example:

```npm install```

```node server.js```

```node mirror.js```

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