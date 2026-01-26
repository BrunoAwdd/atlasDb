#!/bin/bash

# Create a directory for the certificates
mkdir -p certs

# Generate a private key for the CA
openssl genpkey -algorithm RSA -out certs/ca.key

# Generate a self-signed certificate for the CA
openssl req -new -x509 -key certs/ca.key -out certs/ca.pem -subj "/C=US/ST=California/L=San Francisco/O=MyOrg/OU=MyOU/CN=MyCA"

# Generate a private key for the server
openssl genpkey -algorithm RSA -out certs/server.key

# Generate a CSR for the server
openssl req -new -key certs/server.key -out certs/server.csr -subj "/C=US/ST=California/L=San Francisco/O=MyOrg/OU=MyOU/CN=localhost"

# Sign the server certificate with the CA
openssl x509 -req -in certs/server.csr -CA certs/ca.pem -CAkey certs/ca.key -CAcreateserial -out certs/server.pem

# Generate a private key for the client
openssl genpkey -algorithm RSA -out certs/client.key

# Generate a CSR for the client
openssl req -new -key certs/client.key -out certs/client.csr -subj "/C=US/ST=California/L=San Francisco/O=MyOrg/OU=MyOU/CN=MyClient"

# Sign the client certificate with the CA
openssl x509 -req -in certs/client.csr -CA certs/ca.pem -CAkey certs/ca.key -CAcreateserial -out certs/client.pem

# Clean up the CSRs
rm certs/*.csr
