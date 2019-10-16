#!/bin/bash

set -e

mkdir -p certs
cd certs

INSECURE_PW="INSECUREPASSWORDFORTESTINGONLY"

openssl req -new -x509 -days 365 -passout pass:$INSECURE_PW -keyout ca-key.pem -out ca.cer --batch

openssl genrsa -out server.key 4096

openssl req -new -sha256 -key server.key -out server.csr --batch -subj "/CN=localhost"

openssl x509 -req -days 365 -in server.csr --passin pass:$INSECURE_PW -CA ca.cer -CAkey ca-key.pem -CAcreateserial -out server.cer

openssl genrsa -out client.key 4096

openssl req -new -sha256 -key client.key -out client.csr --batch -subj "/CN=client"

openssl x509 -req -days 365 -in client.csr --passin pass:$INSECURE_PW -CA ca.cer -CAkey ca-key.pem -CAcreateserial -out client.cer

# Cleanup

rm *csr
rm *srl
