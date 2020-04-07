var PROTO_PATH = __dirname + '/' + (process.env.PROTO_FILE === undefined ? '../../proto/service.proto' : process.env.PROTO_FILE);
var grpc = require('grpc');
var protoLoader = require('@grpc/proto-loader');
var packageDefinition = protoLoader.loadSync(
    PROTO_PATH,
    {keepCase: true,
     longs: String,
     enums: String,
     defaults: true,
     oneofs: true
    });
var protoDescriptor = grpc.loadPackageDefinition(packageDefinition).kubeware;

var server = new grpc.Server();
server.addProtoService(protoDescriptor.Middleware.service, {
    HandleRequest: handleRequest,
    HandleResponse: handleResponse
});

server.bind('0.0.0.0:17002', grpc.ServerCredentials.createInsecure());
server.start();


function handleRequest(obj, callback) {    
    if (obj.request.uri.indexOf("/v2/endpoint") === 0) {
        let newBody = JSON.parse(obj.request.body);

        if (newBody.status === "COMPLETED") {
            newBody.status = 0;
        } else {
            newBody.status = 1;
        }

        callback(null, {
            status: 'SUCCESS',
            addedHeaders: [ ],
            removedHeaders: [ ],
            body: {
                value: JSON.stringify(newBody)
            },
            statusCode: null
        });    

        return;
    }

    callback(null, {
        status: 'SUCCESS',
        addedHeaders: [ ],
        removedHeaders: [ ],
        body: null,
        statusCode: null
    });    
}

function handleResponse(obj, callback) {
    callback(null, {
        status: 'SUCCESS',
        addedHeaders: [],
        removedHeaders: [],
        body: null,
        statusCode: null
    });
}
