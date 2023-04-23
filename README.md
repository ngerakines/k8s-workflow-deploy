# k8s-workflow-deploy

# TODO

* [ ] Write a plan to support workflow changes while there are active or queued workflow jobs
* [ ] Think about a workflow checksum use tracker
* [ ] Think about workflow enabled, paused, and drain status
* [ ] Think about namespace selectors

# Selection

All deployments must have the `workflow-deploy.ngerakines.me/workflow` annotation. This is used as a practical way to prevent resources from spanning multiple workflows.

Additionally, the `workflow-deploy.ngerakines.me/enabled` annotation must be set to `true` in the resource namespace.

# Grouping

The `groupByNamespace` and `groupAnnotations` annotations are used to group resources in a workflow. By default, the `groupByNamespace` is True and the `groupAnnotations` is empty.

* To group resources by namespace, don't change anything.
* To group resources by namespace and annotation, set `groupByNamespace` to True and add the annotation to `groupAnnotations`.
* To create a group that spans multiple annotations, set `groupByNamespace` to False and add the annotations to `groupAnnotations`.

```
spec:
  groupByNamespace: true
  groupAnnotations:
  - workflow-deploy.ngerakines.me/group
```

# Workflow Changes

When a workflow resource is encountered, a checksum is created of the workflow resource and is matched against all of the targets it applies to.
