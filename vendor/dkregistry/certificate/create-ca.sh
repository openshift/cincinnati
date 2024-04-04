#!/bin/bash

set -e

pushd output

../cfssl gencert \
    -config ../profiles.json \
    -initca ../ca.json \
    | ../cfssljson -bare ca

popd

