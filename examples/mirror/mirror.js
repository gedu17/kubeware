const bodyParser = require('body-parser')
const express = require('express')
const app = express()
const port = 17003

app.use(bodyParser.json());

app.get('*', function (req, res, next) {
    console.log('Request received in mirror.');
    res.sendStatus(200);
});

app.listen(port, () => console.log(`listening on port ${port}!`))