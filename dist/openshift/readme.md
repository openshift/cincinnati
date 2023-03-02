# Deploying Cincinnati using OpenShift Templates 

## Create Cincinnati credentials secret 
Create Cincinnati credentials secret with GitHub token to scrape graph-data repository
```yaml
kind: Secret
apiVersion: v1
metadata:
  name: cincinnati-credentials
  namespace: cincinnati
data:
  github_token.key: <GITHUB_TOKEN_IN_BASE64>
type: Opaque
```

## Deploying Cincinnati
### On OpenShift clusters
```shell
oc create -f cincinnati-deployment.yaml
```

### On other Kubernetes distribution 
To deploy OpenShift templates on non OpenShift Kubernetes clusters, you need to process the 
OpenShift template.
```shell
oc process -f cincinnati-deployment.yaml > cincinnati-processed.json
```
After processing the Cincinnati template, it can be applied to any Kubernetes distribution
including OpenShift
```shell
kubectl apply -f cincinnati-processed.json
```

## Accessing Cincinnati
You need to create a route to access Cincinnati. 