#!/bin/bash

set -e

pushd output

../cfssl gencert \
    -ca ca.pem \
    -ca-key ca-key.pem \
    -config ../profiles.json \
    -profile=server \
    ../localhost.json \
    | ../cfssljson -bare localhost

cat localhost.pem ca.pem > localhost.crt

openssl \
    pkcs8 \
    -topk8 \
    -inform PEM \
    -outform PEM \
    -nocrypt \
    -in localhost-key.pem \
    -out localhost-key-pkcs8.pem

popd

