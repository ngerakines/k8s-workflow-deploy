# Integration Testing

These are tools and components used to test different scenarios based on the requirements.

# Overview

* `k8s-resources/foo.yml` -- The foo namespace and deployment. Has reasonable started and ready probe defaults.

# Getting Started

First, grab the containers used for the integration tests and create the used lables:

```bash
docker pull ghcr.io/ngerakines/okay:1.0.0
docker tag ghcr.io/ngerakines/okay:1.0.0 localhost:5000/okay:1.0.0-1 && docker push localhost:5000/okay:1.0.0-1
docker tag ghcr.io/ngerakines/okay:1.0.0 localhost:5000/okay:1.0.0-2 && docker push localhost:5000/okay:1.0.0-2
docker tag ghcr.io/ngerakines/okay:1.0.0 localhost:5000/okay:1.0.0-3 && docker push localhost:5000/okay:1.0.0-3
docker tag ghcr.io/ngerakines/okay:1.0.0 localhost:5000/okay:1.0.0-4 && docker push localhost:5000/okay:1.0.0-4
docker tag ghcr.io/ngerakines/okay:1.0.0 localhost:5000/okay:1.0.0-5 && docker push localhost:5000/okay:1.0.0-5
```
