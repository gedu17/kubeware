var PROTO_PATH = __dirname + '/../../proto/service.proto';
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
    let authnHeader = obj.request.headers.filter(x => x.name.toLowerCase() === "authorization");

    if (authnHeader.length === 0) {
        callback(null, {
            status: 'STOP',
            addedHeaders: [ ],
            removedHeaders: [ ],
            body: {
                value: "No credentials"
            },
            statusCode: {
                value: 401
            }
        });    

        return;
    }

    let encodedData = authnHeader[0].value.substring("Basic ".length);
    let userData = Buffer.from(encodedData, 'base64').toString('ascii').split(':');

    if (userData[0] === "admin" && userData[1] === "password123") {
        callback(null, {
            status: 'SUCCESS',
            addedHeaders: [
                {
                    'name': 'user',
                    'value': 'admin'
                }
            ],
            removedHeaders: ['authorization'],
            body: null,
            statusCode: null
        });    
    } else {
        callback(null, {
            status: 'STOP',
            addedHeaders: [ ],
            removedHeaders: [ ],
            body: {
                value: "Invalid credentials"
            },
            statusCode: {
                value: 401
            }
        });    
    }    
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
