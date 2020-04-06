const bodyParser = require('body-parser')
const express = require('express')
const app = express()
const port = 17001

app.use(bodyParser.json());

app.post('/v1/endpoint', function (req, res, next) {
    console.log(req.body);
    
    if (req.body.status === 'COMPLETED') {
        res.send(JSON.stringify({
            id: 32121
        }));
    } else {
        res.sendStatus(201);
    }
});

app.post('/v2/endpoint', function (req, res, next) {
    console.log(req.body);

    if (req.body.status === 0) {
        res.send(JSON.stringify({
            id: 32121
        }));
    } else {
        res.sendStatus(201);
    }
});

app.listen(port, () => console.log(`listening on port ${port}!`))