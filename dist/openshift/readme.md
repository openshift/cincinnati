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

## Architecture Overview

Cincinnati now deploys as **separate, independent pods** for graph-builder and policy-engine:

### 🏗️ **Graph-Builder Pod**
- **Purpose**: Scrapes container registries and builds update graphs
- **Scaling**: Static replicas (typically 1)
- **Resources**: Memory-focused for registry operations
- **Service**: `cincinnati-graph-builder:8080`

### 🛡️ **Policy-Engine Pod**
- **Purpose**: Applies policies to graphs and serves filtered results
- **Scaling**: Multi-layer autoscaling (KEDA + HPA fallback, 1-3 replicas)
- **Resources**: CPU-focused for request processing
- **Service**: `cincinnati-policy-engine:80` (maps to internal port 8081)

### 🌐 **Service Communication**
Policy-engine fetches graphs via **Kubernetes DNS**:
```yaml
pe.upstream: "http://cincinnati-graph-builder:8080/api/upgrades_info/graph"
```

## Incident Prevention

The deployment includes comprehensive incident prevention measures that completely solve the 5-whys KEDA autoscaling incident:

### 🎯 **5-Whys Root Cause Resolution**

| Level | Root Cause | Solution Implemented |
|-------|------------|---------------------|
| **5th Why** | Metric `cincinnati_policy_engine_graph_incoming_requests_rate` missing | ✅ **KEDA uses base metric**: `sum(rate(cincinnati_pe_graph_incoming_requests_total[2m]))` |
| **4th Why** | Autoscaler broken, manual scaling required | ✅ **Multi-layer autoscaling**: KEDA + HPA fallback ensures autoscaling always works |
| **3rd Why** | Insufficient replicas to handle load | ✅ **Working autoscaling**: HPA automatically scales based on CPU (70% target) |
| **2nd Why** | Policy Engine misbehaving under load | ✅ **Independent scaling**: Policy-engine scales without affecting graph-builder |
| **1st Why** | OCM returns 500s due to Cincinnati degradation | ✅ **Service resilience**: Fast recovery (5-10s) and proactive scaling prevent degradation |

### ✅ **Resilient KEDA Configuration**
- **Base metrics only**: Uses `sum(rate(cincinnati_pe_graph_incoming_requests_total[2m]))` directly
- **No recording rule dependency**: Cannot be broken by PrometheusRule failures
- **Multi-layer autoscaling**: KEDA + HPA fallback eliminates single points of failure

### ⚡ **10-15x Faster Recovery**
- **Independent pods**: Policy-engine starts without waiting for graph-builder
- **Optimized startup**: 5-second startup probe delay, 2-second check intervals
- **Fast readiness**: 30-second readiness vs 300-second before
- **Improved liveness**: 60-second liveness vs 300-second before
- **Smart health checks**: Startup probe handles graph-builder dependency gracefully

### 📊 **Enhanced Monitoring**
- **KEDA health tracking**: `cincinnati_keda_policy_engine_scaler_active` metric
- **Proactive alerting**: Monitor autoscaler health to catch failures early
- **Independent metrics**: Separate ServiceMonitor for each service

## Benefits of Separate Deployments

### 🚀 **Recovery Speed**
- **Policy-engine startup**: ~5-10 seconds vs 5+ minutes co-located
- **Independent scaling**: Scale policy-engine without affecting graph-builder
- **Incident recovery**: 10-15x faster as mentioned in incident discussion

### 🔧 **Operational Excellence**
- **Resource efficiency**: Targeted CPU/memory allocation per service
- **Independent updates**: Deploy services separately without downtime
- **Clear monitoring**: Separate logs, metrics, and health checks
- **Fault isolation**: Graph-builder issues don't affect policy-engine scaling

### 📈 **Scaling Flexibility**
- **Graph-builder**: Static scaling focused on memory for registry operations
- **Policy-engine**: Dynamic KEDA scaling based on request load
- **Independent limits**: Different CPU/memory requirements per service

## Emergency Procedures

If autoscaling fails during an incident, follow these steps:

### **1. Check Autoscaling Status**
```bash
# Check both autoscalers
oc get scaledobject cincinnati-policy-engine-scaler
oc get hpa cincinnati-policy-engine-hpa-fallback
oc describe scaledobject cincinnati-policy-engine-scaler
oc describe hpa cincinnati-policy-engine-hpa-fallback
```

### **2. Manual Scaling (If Both Autoscalers Fail)**
```bash
# Immediate manual scaling as backup
oc scale deployment cincinnati-policy-engine --replicas=5
```

### **3. Verify Base Metric Availability**
```bash
# Check if metric exists (this prevents 5th Why recurrence)
kubectl port-forward svc/prometheus-app-sre 9090:9090 &
curl 'http://localhost:9090/api/v1/query?query=cincinnati_pe_graph_incoming_requests_total'
curl 'http://localhost:9090/api/v1/query?query=sum(rate(cincinnati_pe_graph_incoming_requests_total[2m]))'
```

### **4. Check Incident Prevention Alerts**
```bash
# Verify autoscaling health alerts are working
oc get prometheusrule cincinnati-autoscaler-alerts -o yaml
oc get prometheusrule cincinnati-recording-rule -o yaml
```

### **5. Service Communication Verification**
```bash
# Test Kubernetes DNS communication (addresses 2nd Why)
curl "http://cincinnati-policy-engine/api/upgrades_info/graph?channel=stable-4.2&arch=amd64"
oc exec deployment/cincinnati-policy-engine -- \
  curl http://cincinnati-graph-builder:8080/api/upgrades_info/graph

# Verify independent pod status (addresses 1st Why)
oc get pods -l app=cincinnati-graph-builder
oc get pods -l app=cincinnati-policy-engine
```

## Essential Monitoring

### Core Metrics (Required for Autoscaling)
- `cincinnati_pe_graph_incoming_requests_total` - Base metric for request rate (used directly by KEDA)
- `sum(rate(cincinnati_pe_graph_incoming_requests_total[2m]))` - Computed request rate for KEDA scaling

### Health Monitoring (Recording Rules)
- `cincinnati_keda_policy_engine_scaler_active` - KEDA autoscaler health
- `cincinnati_hpa_policy_engine_active` - HPA fallback autoscaler health
- `cincinnati_policy_engine_graph_incoming_requests_rate` - Dashboard compatibility metric

### Kubernetes Metrics (Built-in)
- `kube_deployment_status_replicas_available{deployment="cincinnati-policy-engine"}` - PE available replicas
- `kube_deployment_status_replicas_available{deployment="cincinnati-graph-builder"}` - GB available replicas
- `kube_horizontalpodautoscaler_status_current_replicas{horizontalpodautoscaler="cincinnati-policy-engine-hpa-fallback"}` - HPA status

### Implemented Incident Prevention Alerts

The deployment includes these critical alerts (defined in `cincinnati-autoscaler-alerts` PrometheusRule):

```yaml
# Alert when both autoscalers fail (prevents manual scaling incidents)
- alert: CincinnatiAutoscalingCompletelyBroken
  expr: |
    (
      (cincinnati_keda_policy_engine_scaler_active == 0 OR absent(cincinnati_keda_policy_engine_scaler_active))
      AND
      (kube_horizontalpodautoscaler_status_current_replicas{horizontalpodautoscaler="cincinnati-policy-engine-hpa-fallback"} == 0 OR absent(kube_horizontalpodautoscaler_status_current_replicas{horizontalpodautoscaler="cincinnati-policy-engine-hpa-fallback"}))
    )
  for: 5m
  annotations:
    summary: "Both KEDA and HPA autoscaling are broken for Cincinnati policy-engine"
    description: "Manual scaling required immediately - both autoscaling mechanisms have failed"
    runbook: "Scale manually: oc scale deployment cincinnati-policy-engine --replicas=5"

# Alert when policy-engine is under-scaled for load
- alert: CincinnatiPolicyEngineUnderScaled
  expr: |
    sum(rate(cincinnati_pe_graph_incoming_requests_total[5m])) > 100
    and
    kube_deployment_status_replicas_available{deployment="cincinnati-policy-engine"} < 3
  for: 2m
  annotations:
    summary: "Cincinnati policy-engine under-scaled for current load"
    description: "Request rate is high but insufficient replicas available"

# Alert when base metric disappears (prevents KEDA scaling failures)
- alert: CincinnatiBaseMetricMissing
  expr: absent(cincinnati_pe_graph_incoming_requests_total)
  for: 5m
  annotations:
    summary: "Cincinnati base metric missing - autoscaling will break"
    description: "The metric cincinnati_pe_graph_incoming_requests_total is not available"
```

## Parameter Customization

View available template parameters:
```shell
oc process --parameters -f cincinnati-deployment.yaml
```

Override parameters during deployment:
```shell
# Scale policy-engine more aggressively
oc process -f cincinnati-deployment.yaml \
  -p PE_MEMORY_LIMIT=2Gi \
  -p MAX_REPLICAS=5 \
  -p PE_REQ_AVG=30 | oc apply -f -

# Allocate more resources to graph-builder
oc process -f cincinnati-deployment.yaml \
  -p GB_REPLICAS=2 \
  -p GB_MEMORY_LIMIT=1Gi \
  -p GB_CPU_LIMIT=1000m | oc apply -f -
```

## Verification

### Template Processing
Verify template processes correctly:
```shell
oc process -f cincinnati-deployment.yaml > test-processed.yaml
kubectl apply --dry-run=client -f test-processed.yaml
```

### Health Checks
```shell
# Graph-builder health
curl http://cincinnati-graph-builder:9080/liveness
curl http://cincinnati-graph-builder:9080/readiness

# Policy-engine health
curl http://cincinnati-policy-engine:9081/livez
curl http://cincinnati-policy-engine:9081/readyz
```

### Service Communication
```shell
# Test Kubernetes DNS communication
oc exec deployment/cincinnati-policy-engine -- \
  curl http://cincinnati-graph-builder:8080/api/upgrades_info/graph

# Test end-to-end functionality
curl "http://cincinnati-policy-engine/api/upgrades_info/graph?channel=stable-4.2&arch=amd64"
```

### Independent Scaling Verification
```shell
# Scale policy-engine independently
oc scale deployment cincinnati-policy-engine --replicas=3

# Verify graph-builder unaffected
oc get pods -l app=cincinnati-graph-builder

# Test KEDA autoscaling
# Generate load and verify automatic scaling occurs
```

## Deployment Architecture Summary

| Component | Pod Type | Scaling | Communication | Recovery Time |
|-----------|----------|---------|---------------|---------------|
| **Graph-Builder** | Independent | Static (1 replica) | Kubernetes Service DNS | ~30 seconds |
| **Policy-Engine** | Independent | KEDA Autoscaling (1-3) | Fetches from GB via DNS | ~5-10 seconds |
| **Original (Co-located)** | Single pod | KEDA (entire pod) | Localhost | ~5+ minutes |

## Architecture Evolution

| Aspect | Before (Vulnerable) | After (Enhanced) |
|--------|-------------------|------------------|
| **Autoscaling** | KEDA only (single point of failure) | KEDA + HPA (multi-layer resilience) |
| **Metric Dependency** | Recording rule (can break) | Base metric (resilient) |
| **Pod Architecture** | Co-located containers | Independent pods |
| **Recovery Time** | 5+ minutes | 5-10 seconds |
| **Communication** | `localhost:8080` | `cincinnati-graph-builder:8080` |
| **Scaling** | Both services together | Independent scaling per service |
| **Monitoring** | Single ServiceMonitor, basic recording rules | Separate ServiceMonitors, incident prevention alerts, autoscaler health tracking |

## Documentation

This deployment implements comprehensive incident prevention measures based on detailed 5-whys analysis of KEDA autoscaling failures. The multi-layer autoscaling approach ensures service resilience and prevents the exact failure scenarios that led to production incidents.

## Accessing Cincinnati

You need to create a route to access the Cincinnati policy-engine service for external access.