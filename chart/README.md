
# workflow-deploy

A Helm chart for the workflow-deploy daemon.

Usage:

    $ helm upgrade --install --set=image.repository=localhost:5000/k8s-workflow-deploy --set=image.tag=d669ff5eb777c08245dc2c15c30a526cc48daf47 default ./chart/

## Configuration

The following table lists the configurable parameters of the workflow-deploy chart and their default values.

| Parameter                | Description             | Default        |
| ------------------------ | ----------------------- | -------------- |
| `replicaCount` |  | `1` |
| `image.repository` |  | `"ngerakines/workflow-deploy"` |
| `image.pullPolicy` |  | `"IfNotPresent"` |
| `image.tag` |  | `"latest"` |
| `imagePullSecrets` |  | `[]` |
| `nameOverride` |  | `""` |
| `fullnameOverride` |  | `""` |
| `serviceAccount.create` |  | `true` |
| `serviceAccount.annotations` |  | `{}` |
| `serviceAccount.name` |  | `""` |
| `resources.requests.cpu` |  | `"100m"` |
| `resources.requests.memory` |  | `"128Mi"` |
| `nodeSelector` |  | `{}` |
| `tolerations` |  | `[]` |
| `affinity` |  | `{}` |
| `log_level` |  | `"k8s_workflow_deploy=warn,error"` |
| `run_mode` |  | `"production"` |
| `local_config` |  | `{}` |
