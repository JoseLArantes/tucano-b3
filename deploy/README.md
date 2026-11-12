# Kubernetes Deployment (tucano-b3)

Deploy and manage `tucano-b3` with **Helm** and **Argo Rollouts**, following the same layout as `tmqa-backend`.

Public URL: **https://b3-tucano.beakcloud.com**  
Namespace: **tucano-services**  
Image: **ghcr.io/joselarantes/tucano-b3:latest**  
Cluster: **bos-k** (Argo CD)

---

## 1. Prerequisites

### A. Argo Rollouts

```bash
kubectl create namespace argo-rollouts
kubectl apply --server-side --force-conflicts -n argo-rollouts \
  -f https://github.com/argoproj/argo-rollouts/releases/latest/download/install.yaml
```

### B. Cluster facts (evaluated on `bos-k`)

| Item | Value |
|---|---|
| Destination namespace | `tucano-services` |
| Ingress class | `nginx` (same pattern as tmqa) |
| Registry pull secret | `registry-cred` in `tucano-services` |
| TLS | Terminates upstream of the Ingress (no cert-manager Certificate in-chart) |
| Rollouts CRD | installed (`argo-rollouts` namespace) |
| Git source | Public HTTPS — no Argo CD repository SSH secret required |

### C. Kubectl Argo Rollouts plugin (optional)

```bash
brew install argoproj/tap/kubectl-argo-rollouts
```

---

## 2. Network path

```
[Client] https://b3-tucano.beakcloud.com
        │
        ▼
[Edge / Istio public-gateway + nginx Ingress]
        │
        ▼
[Ingress: tucano-b3-ingress]  (ingressClassName: nginx)
        │
        ▼
[Service: tucano-b3]  ClusterIP :8080
        │
        ▼
[Pods] managed by Argo Rollout `tucano-b3`
```

---

## 3. Secrets

### Registry pull secret

`image.pullSecrets` references `registry-cred` in `tucano-services`.

### Application secret

Create `tucano-b3-secret` before the first sync:

```bash
cp deploy/secret.example.yaml /tmp/tucano-b3-secret.yaml
# edit TUCANO_B3_TOKEN
kubectl apply -f /tmp/tucano-b3-secret.yaml -n tucano-services
```

Vault/ESO for app secrets is optional (`vault.enabled: false` by default).

---

## 4. Helm (manual)

```bash
# Lint / dry-run
helm lint deploy/helm
helm template tucano-b3 deploy/helm -n tucano-services \
  -f deploy/helm/values.yaml -f deploy/helm/values-production.yaml

# Install / upgrade
helm upgrade --install tucano-b3 deploy/helm -n tucano-services \
  -f deploy/helm/values.yaml -f deploy/helm/values-production.yaml
```

Canary rollout after an image bump:

```bash
helm upgrade tucano-b3 deploy/helm -n tucano-services \
  -f deploy/helm/values.yaml -f deploy/helm/values-production.yaml \
  --set image.tag="0.1.1"

kubectl argo rollouts get rollout tucano-b3 -n tucano-services --watch
```

---

## 5. Argo CD

Manifests live under `deploy/argocd/`:

| File | Purpose |
|---|---|
| `appproject.yaml` | AppProject `tucano-services` |
| `application-production.yaml` | Application `tucano-b3` → namespace `tucano-services` |

The GitHub repository is **public**. The Application uses
`https://github.com/JoseLArantes/tucano-b3.git` — no Argo CD repository credential or SSH ExternalSecret is required.

Apply (from a cluster that runs Argo CD):

```bash
kubectl apply -f deploy/argocd/appproject.yaml
kubectl apply -f deploy/argocd/application-production.yaml
```

Source path: `deploy/helm` with `values.yaml` + `values-production.yaml`.  
Destination: namespace `tucano-services`, project `tucano-services`.

Automated sync is commented out until the first manual sync is green (same tip as tmqa-backend).

---

## 6. Rollout operations

```bash
kubectl argo rollouts get rollout tucano-b3 -n tucano-services --watch
kubectl argo rollouts promote tucano-b3 -n tucano-services
kubectl argo rollouts abort tucano-b3 -n tucano-services
kubectl argo rollouts restart tucano-b3 -n tucano-services
```

---

## 7. Troubleshooting

```bash
kubectl describe rollout tucano-b3 -n tucano-services
kubectl get pods -n tucano-services -l app=tucano-b3
kubectl logs -n tucano-services -l app=tucano-b3 -f
kubectl get ingress -n tucano-services
kubectl describe ingress tucano-b3-ingress -n tucano-services
```
