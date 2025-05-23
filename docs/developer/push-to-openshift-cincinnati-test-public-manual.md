# Repository openshift-cincinnati-test-public-manual

The image repository [quay.io/openshift-ota/openshift-cincinnati-test-public-manual](https://quay.io/repository/openshift-ota/openshift-cincinnati-test-public-manual?tab=tags) is created for testing as
a release repo for `graph-builder` to scrape. 

## Mirror

The simplest way to make an image in the `openshift-cincinnati-test-public-manual` repo is to mirror one from `quay.io/openshift-release-dev/ocp-release`. E.g.,

```console
$ oc image mirror --keep-manifest-list --registry-config=/tmp/docker.json --max-per-registry=10 quay.io/openshift-release-dev/ocp-release:4.18.11-multi=quay.io/openshift-ota/openshift-cincinnati-test-public-manual:4.18.11-multi
```

We can mirror all the interesting cases to `openshift-cincinnati-test-public-manual` so that using the repo as the scraping target in CI e2e tests will keep `cincinnati` away from regression.

## Build and Push

For example, we can build and push `quay.io/openshift-ota/openshift-cincinnati-test-public-manual:4.18.6` with the following files in the current folder:

```console
$ for file in ./*; do echo "$file"; cat "$file"; done
./Dockerfile
FROM alpine
CMD ["echo", "Hello World!!"]
LABEL io.openshift.release="4.18.6"
COPY release-metadata /release-manifests/release-metadata
./release-metadata
{
  "kind": "cincinnati-metadata-v0",
  "version": "4.18.6",
  "previous": [
    "4.18.3"
  ],
  "metadata": {
    "url": "https://access.redhat.com/errata/RHSA-202X:ABAB"
  }
}
```

with `buildah`:

```console
$ buildah bud --format docker -t quay.io/openshift-ota/openshift-cincinnati-test-public-manual:4.18.6 .
$ buildah push --authfile ~/.docker/config.json quay.io/openshift-ota/openshift-cincinnati-test-public-manual:4.18.6
```


## Format of Image's Manifest and Metadata

We have to use `--format docker` to build the image when using `buildah`. Otherwise, the default `oci` would lead to an image that `graph-builder` yields "unknown media type ManifestV2S1" error to and then ignores. See [the supported media types](https://github.com/camallo/dkregistry-rs/blob/3e242ee9e39646da6ff4a886e080085cc1810d37/src/v2/manifest/mod.rs#L74-L96).

We cannot use `podman` to build the image because `podman build --format docker` does not work. See [podman/issues/21294](https://github.com/containers/podman/issues/21294).

```console
$ skopeo inspect --raw docker://quay.io/openshift-ota/openshift-cincinnati-test-public-manual:4.18.12-x86_64-podman | jq{
  "schemaVersion": 2,
  "mediaType": "application/vnd.oci.image.manifest.v1+json",
  ...
}

$ docker manifest inspect quay.io/openshift-ota/openshift-cincinnati-test-public-manual:4.18.12-x86_64-podman
{
  "schemaVersion": 2,
  "mediaType": "application/vnd.oci.image.manifest.v1+json",
  ...
}
```

Note that `podman manifest inspect` the above image led to errors. So does `buildah`. However, either `skopeo` or `docker` works.

Besides `buildah`, `openshift/build` generates the images that `graph-builder` is happy with. The following table contains the testing results about these build tools.

| command         | version                                                       | os                                    | install                     | working |
|-----------------|---------------------------------------------------------------|---------------------------------------|-----------------------------|---------|
| buildah         | buildah version 1.39.2 (image-spec 1.1.0, runtime-spec 1.2.0) | Fedora Linux 40 (Workstation Edition) | sudo dnf -y install buildah | yes     |
| skopeo          | skopeo version 1.18.0                                         | Fedora Linux 40 (Workstation Edition) | sudo dnf -y install skopeo  | yes     |
| openshift-build | OSD 4.18.11                                                   | n/a                                   | n/a                         | yes     |
| podman          | podman version 5.4.2                                          | macOS Sequoia 15.4.1 (24E263)         | brew  install podman        | no      |
