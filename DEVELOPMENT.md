# Development

## Minikube

Start minikube with the following command:

    $ minikube start

Reset if things go wrong:

    $ minikube delete
    $ minikube start

Run a registry:

    $ minikube addons enable registry

On windows, you need to run the following command to use the registry:

    $ minikube kubectl -- port-forward --namespace kube-system service/registry 5000:80
    $ docker run --rm -it --network=host alpine ash -c "apk add socat && socat TCP-LISTEN:5000,reuseaddr,fork TCP:host.docker.internal:5000"

## Helm setup

    $ kubectl apply -f ./k8s-resources/ns.yml
    $ kubectl apply -f ./k8s-resources/serviceaccount.yml
    $ kubectl apply -f ./k8s-resources/rbac.yml
    $ kubectl apply -f ./k8s-resources/deployment.yml

## Build and deploy

Build the container:

    $ docker build -t localhost:5000/k8s-workflow-deploy:`git rev-parse HEAD` .

4. Build and push the container.

```bash
docker build --build-arg GIT_HASH=`git rev-parse HEAD` -t localhost:5000/k8s-workflow-deploy:`git rev-parse HEAD` .
docker tag localhost:5000/k8s-workflow-deploy:`git rev-parse HEAD` localhost:5000/k8s-workflow-deploy:latest
docker push localhost:5000/k8s-workflow-deploy:latest
docker push localhost:5000/k8s-workflow-deploy:`git rev-parse HEAD`
helm upgrade --install --set=fullnameOverride=workflow-deploy --set=image.repository=localhost:5000/k8s-workflow-deploy --set=image.tag=`git rev-parse HEAD` workflow-deploy ./chart/
```

## Examples

    $ kubectl apply -f ./k8s-resources/workflow_standard.yml
