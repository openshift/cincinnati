# RapidAST Security Scanning

Run automated security scans against OpenShift APIs using RapidAST container.

## Quick Start

```bash
# Pull the container
podman pull quay.io/redhatproductsecurity/rapidast:latest

# Create results directory
mkdir -p results

# Run security scan
podman run --rm \
  -v $(pwd)/rapidast-config.yaml:/tmp/config.yaml \
  -v $(pwd)/results:/opt/rapidast/results \
  quay.io/redhatproductsecurity/rapidast:latest \
  --config /tmp/config.yaml
```

## Configuration

- `rapidast-config.yaml`: Scan configuration for OpenShift upgrades API
- Disables only cookie/session rules (appropriate for stateless APIs)
- Uses passive scanning (safe for production)
