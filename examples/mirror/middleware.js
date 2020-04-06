var PROTO_PATH = __dirname + '/../../proto/service.proto';
var grpc = require('grpc');
var http = require('http');
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
    callback(null, {
        status: 'SUCCESS',
        addedHeaders: [ ],
        removedHeaders: [ ],
        body: null,
        statusCode: null
    });   
    
    const options = {
        hostname: '127.0.0.1',
        port: 17003,
        path: obj.request.uri,
        method: obj.request.method
    };          

    const req = http.request(options);
          
    req.end();     
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
