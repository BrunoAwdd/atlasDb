#!/bin/bash

mkdir -p certs
cd certs

# Create CA
openssl req -x509 -newkey rsa:4096 -days 365 -nodes \
  -keyout ca.key -out ca.pem \
  -subj "/C=US/ST=California/L=San Francisco/O=MyOrg/OU=MyOU/CN=MyCA"

# Create Server Cert
openssl req -newkey rsa:4096 -nodes \
  -keyout server.key -out server.csr \
  -subj "/C=US/ST=California/L=San Francisco/O=MyOrg/OU=MyOU/CN=localhost"

# Sign Server Cert with V3 extensions
cat > server.ext << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
EOF

openssl x509 -req -in server.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -out server.pem -days 365 -sha256 -extfile server.ext

# Create Client Cert
openssl req -newkey rsa:4096 -nodes \
  -keyout client.key -out client.csr \
  -subj "/C=US/ST=California/L=San Francisco/O=MyOrg/OU=MyOU/CN=MyClient"

# Sign Client Cert with V3 extensions
cat > client.ext << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = clientAuth
EOF

openssl x509 -req -in client.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -out client.pem -days 365 -sha256 -extfile client.ext

# Cleanup
rm *.csr *.srl server.ext client.ext
