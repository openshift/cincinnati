#!/bin/bash

version=1.6.1

rm -f cfssl
wget -O cfssl https://github.com/cloudflare/cfssl/releases/download/v${version}/cfssl_${version}_linux_amd64
chmod +x cfssl

rm -f cfssljson
wget -O cfssljson https://github.com/cloudflare/cfssl/releases/download/v${version}/cfssljson_${version}_linux_amd64
chmod +x cfssljson

