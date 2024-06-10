# banshee

A small hack to improve visibility if your pod has been killed by the OOM killer.

## Overview

Normally, when a pod exceeds its memory limits and it is killed, you can diagnose this by running ``kubectl describe pod``:

```
    Last State:     Terminated
      Reason:       OOMKilled
      Exit Code:    137
```

In the events associated with the pod, you will only see that the pod is restarted but not the reason behind it.

banshee monitors for killed pods and creates an event for the pod, so you can see it in your monitoring system if you forward events to the monitoring system.

```
Events:
  Type     Reason      Age                     From     Message
  ----     ------      ----                    ----     -------
  Normal   OOMKilling  161s                    banshee  Pod stress-deployment-c4c6c8bbb-tfgjj in namespace default was OOMKilled.
```

Note that an event is recorded for every restart, so you will see several events for restart loops.

## Installation

banshee was tested against Kubernetes server version 1.29. You can install it into your cluster using helm:

```
helm repo add eickler-charts https://eickler.github.io/charts/
helm repo update
helm install banshee eickler-charts/banshee
kubectl get deployment banshee
```

Unfortunately, Github does not permit unauthenticated access to the Github Container Registry. Create a personal access token (classic) with the permission read:packages. Install the token as secret into your cluster to enable download of the images:

```
kubectl create secret docker-registry regcred --docker-server=ghcr.io --docker-username=GITHUB_USERNAME --docker-password=GITHUB_TOKEN
```
