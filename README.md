# k8s-workflow-deploy

This is a Kubernetes controller that updates groups of deployments using Workflow definitions.

Use case: You have a bunch of the same deployments across multiple namespaces that you want to update safely in parallel.

# Workflow CRD

```yaml
---
apiVersion: workflow-deploy.ngerakines.me/v1alpha
kind: Workflow
metadata:
  name: tenants
spec:
  version: "1.0.0"
  debounce: 90
  namespaces: ["foo", "bar", "baz"]
  supression: []
  steps:
  - actions:
    - action: scale_down
      targets:
      - resource: Deployment
        name: worker
        containers: ["app"]
  - actions:
    - action: update_deployment
      targets:
      - resource: Deployment
        name: app
        containers: ["app"]
      - resource: Deployment
        name: worker
        containers: ["app"]
      - resource: Deployment
        name: api
        containers: ["api"]
  - actions:
    - action: restore_replica_count
      targets:
      - resource: Deployment
        name: worker
        containers: ["app"]
```

# Grouping and selection

Resources are updated in groups using their kubernetes namespace as the selector.

For example using the above Workflow resource, if the app, worker, and api deployments all exist in the foo, bar, and baz namespaces, then the "tenants" workflow would have 3 groups that are updated independantly of eachother.

# Roadmap

* [ ] Add support for namespace selection using annotations.

  This would remove the `namespaces` attribute from the Workflow resource and instead look for the `workflow-deploy.ngerakines.me/enabled` and `workflow-deploy.ngerakines.me/workflow` annotations on namespaces.
