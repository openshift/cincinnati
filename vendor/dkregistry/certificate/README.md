
# Certificate Generation and Persistence for Tests

While it is possible to automagically generate certificates using the [rcgen](https://github.com/est31/rcgen)
crate, that library (as of version 0.10.0) has a dependency on the [ring](https://github.com/briansmith/ring)
crate, which has a non-trivial set of licenses.

To avoid potential problems with the licenses applying to `ring`, `rcgen` is not used to generate
test certificates.

## Generating and Persisting Test Certificates

The tests require a self-signed certificate authority, and a private key / server certificate pair signed by
that same CA.

Certificates are defined in json files, generated using [cfssl](https://github.com/cloudflare/cfssl), and
committed into git.

### Install `cfssl` and Re-generate Certificates

    $ ./download-cfssl.sh
    $ ./create-ca.sh
    $ ./create-localhost.sh

Note: You should not have to regenerate any certificates unless they expire, the ciphers become insecure,
or the certificates otherwise become rejected by future versions of cryptography libraries.

### Definitions

* [profiles.json](profiles.json)
* [ca.json](ca.json)
* [localhost.json](localhost.json)
