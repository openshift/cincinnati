# Grafana Dashboards 

To create a configmap for grafana dashboard or to update existing configmap,
1. Insert the `<dashboard-name>.json` grafana file to `dist/grafana` folder
2. Run `just dashboards` from the root directory to create corresponding
   config maps.
3. The config maps are stored in `/dist/grafana/dashboards` folder
