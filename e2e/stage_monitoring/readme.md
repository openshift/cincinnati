# Purpose
In order to find cincinnati's issue in advance, we suggest to create a monitoring pod on Stage cincinnati, this pod can keep monitoring the OpenAPI and graph API, and will send slack messages when there are issues.

# How to

## Update images
The monitoring pod has below 3 containers:
### cincinnati-monitor
This container will continuesly ping graph API and v1/graph API and check its response, once there are issuces it will send an error mesage to Slack channel.

You can run below command to update its image:


    $ podman build -t quay.io/openshifttest/monitor:latest -f Dockerfile.cincy
    $ podman push quay.io/openshifttest/monitor:latest


### cincinnati-monitor-spec
This container is used to check openapi, once there are spec issues it will send an error mesage to Slack channel.

You can run below command to update its image:


    $ podman build -t quay.io/openshifttest/monitor_digest:latest -f Dockerfile.digest
    $ podman push quay.io/openshifttest/monitor_digest:latest

### cincinnati-monitor-digest
This container aims to verify the bug [Cincinnati should not confuse shards of a multi-arch release with single-arch releases](https://issues.redhat.com/browse/OCPBUGS-56124), if cincinnati returns an invalid shards from different CPU architecture it will send an error mesage to Slack channel.

You can run below command to update its image:


    $ podman build -t quay.io/openshifttest/monitor_spec:latest -f Dockerfile.spec
    $ podman push quay.io/openshifttest/monitor_spec:latest

## deploy

### create a secret
Because we are using Slack workflow to send messages so we should keep the workflow's web request URL secure, we are using secret to store the URL, you can run below command to create the secret:


    $ oc create secret generic slack-workflow --from-literal=SLACK_URL=`The Slack workflow URL` -n cincinnati-quality-experiments

The slack workflow is `stage_cincinnati`, its administrators are @jhou @pmahajan @pmuller @wking @hongkliu @jianl, you can ask them to copy its web request URL, the steps are:
Edit workflow -> edit `Start the workflow` -> in the popup, scroll to bottom -> Click `Copy Link`

You can follow below steps to update the target Slack channel:
Edit workflow -> edit `Then, do these things` -> Select a channel


For more details about Slack workflow, you can refer: https://slack.com/intl/en-gb/features/workflow-automation

### Create monitoring pod

You can run below command to create the monitoring pod:


    $ oc apply -f deployment.yaml

Or run below command to update the pod:

    $ oc replace -f deployment.yaml
