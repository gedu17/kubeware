const express = require('express')
const app = express()
const port = 17001

app.get('*', function (req, res, next) {
    res.send('Hello, ' + req.header('user'));
});

app.listen(port, () => console.log(`listening on port ${port}!`))