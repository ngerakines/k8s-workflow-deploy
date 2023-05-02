# k8s-workflow-deploy

`workflow-deploy` is a Kubernetes controller that updates groups of deployments using Workflow definitions.

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
    supression:
    - "2023-05-02T14:00:00.000000-04:00"
    - "2023-05-05T19:00:0-04:00 2023-05-09T07:00:00-04:00"
  steps:
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
```

# Grouping and selection

Resources are updated in groups using their kubernetes namespace as the selector.

For example using the above Workflow resource, if the app, worker, and api deployments all exist in the foo, bar, and baz namespaces, then the "tenants" workflow would have 3 groups that are updated independantly of eachother.

# Roadmap

* [ ] Add support for namespace selection using annotations.

  This would remove the `namespaces` attribute from the Workflow resource and instead look for the `workflow-deploy.ngerakines.me/enabled` and `workflow-deploy.ngerakines.me/workflow` annotations on namespaces.

* [ ] Relative suppression values.

  This includes support for values like "Friday after 5:00 PM to Monday at 7:00 AM", "Weekdays before 5:00 AM", and "Weekdays after 9:00 PM"
