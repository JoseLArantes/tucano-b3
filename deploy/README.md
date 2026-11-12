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
[Ingress: tucano-b3-ingress]  (ingressClassName: nginx, default 1m body)
[Ingress: tucano-b3-ingress-ingest]  path /api/v1/ingest, proxy-body-size 500m
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

Production (`values-production.yaml`) turns on External Secrets. ESO reads KV v2 path `tucano-services/production/base` on mount `bos-secrets` (`/v1/bos-secrets/data/tucano-services/production/base`) and writes Kubernetes secret `tucano-b3-secret`.

The SecretStore uses the Vault Kubernetes auth role `tucano-services` and the `default` service account in `tucano-services`. It trusts Vault through ConfigMap `vault-ca` in `tucano-services` (`caProvider`), the same CA as `ClusterSecretStore/vault-cluster-store`. That certificate is not in git.

```bash
kubectl get clustersecretstore vault-cluster-store -o jsonpath='{.spec.provider.vault.caBundle}' \
  | base64 -d > /tmp/vault-ca.crt
kubectl create configmap vault-ca -n tucano-services \
  --from-file=ca.crt=/tmp/vault-ca.crt \
  --dry-run=client -o yaml | kubectl apply -f -
rm /tmp/vault-ca.crt
```

`tucano-b3-secret` already exists and is not owned by ESO. Delete it once before the first sync so the ExternalSecret can create it. Pods keep the previous environment until they restart.

```bash
kubectl delete secret tucano-b3-secret -n tucano-services
# after the ExternalSecret reports SecretSynced:
kubectl argo rollouts restart tucano-b3 -n tucano-services
kubectl get externalsecret -n tucano-services
```

`deploy/secret.example.yaml` is only for a cluster where `vault.enabled` is false.

### Object storage (Cloudflare R2)

`POST /api/v1/ingest/bucket` reads COTAHIST `.ZIP` objects from an S3-compatible bucket. It does not run on a timer.

| Variable | Where | Purpose |
|---|---|---|
| `TUCANO_S3_ENDPOINT` | ConfigMap | `https://<account_id>.r2.cloudflarestorage.com` |
| `TUCANO_S3_BUCKET` | ConfigMap | Bucket name |
| `TUCANO_S3_REGION` | ConfigMap | `auto` for R2 |
| `TUCANO_S3_PREFIX` | ConfigMap | Optional key prefix, such as `cotahist/` |
| `TUCANO_S3_FORCE_PATH_STYLE` | ConfigMap | `true` for R2 |
| `TUCANO_S3_ACCESS_KEY_ID` | Secret | R2 access key |
| `TUCANO_S3_SECRET_ACCESS_KEY` | Secret | R2 secret |

Set the non-secret values in `deploy/helm/values-production.yaml` (`config.s3*`). Put `TUCANO_S3_ACCESS_KEY_ID` and `TUCANO_S3_SECRET_ACCESS_KEY` in the Vault secret `tucano-services/production/base`, next to `TUCANO_B3_TOKEN`. ESO copies every key into `tucano-b3-secret`. A call with an empty body uses the configured prefix; `{"prefix":"cotahist/2025/"}` overrides it for that request only.

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

## 5. Deploy updates (Argo CD)

Automated sync is off. To ship a Helm/values change:

1. Commit and push to `master` (the Application `targetRevision`).
2. Refresh and sync the app:

```bash
kubectl apply -f deploy/argocd/application-production.yaml
argocd app get tucano-b3
argocd app sync tucano-b3 --prune
# or in the Argo CD UI: Refresh → Sync
```

3. Watch the canary Rollout finish, then confirm the live pod picked up the change:

```bash
kubectl argo rollouts get rollout tucano-b3 -n tucano-services --watch
kubectl get pods -n tucano-services -l app=tucano-b3 \
  -o jsonpath='{range .items[*]}{.metadata.name}{" "}{.spec.containers[0].resources}{"\n"}{end}'
```

A values-only memory change updates the Rollout only if Argo is allowed to diff `resources`. That path must not be listed under `ignoreDifferences`.

## 6. Argo CD manifests

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

## 7. Rollout operations

```bash
kubectl argo rollouts get rollout tucano-b3 -n tucano-services --watch
kubectl argo rollouts promote tucano-b3 -n tucano-services
kubectl argo rollouts abort tucano-b3 -n tucano-services
kubectl argo rollouts restart tucano-b3 -n tucano-services
```

---

## 8. Troubleshooting

```bash
kubectl describe rollout tucano-b3 -n tucano-services
kubectl get pods -n tucano-services -l app=tucano-b3
kubectl logs -n tucano-services -l app=tucano-b3 -f
kubectl get ingress -n tucano-services
kubectl describe ingress tucano-b3-ingress -n tucano-services
kubectl describe ingress tucano-b3-ingress-ingest -n tucano-services
```

`POST /api/v1/ingest/upload` accepts ZIP archives up to **500 MB** (the application limit). `tucano-b3-ingress-ingest` sets `nginx.ingress.kubernetes.io/proxy-body-size: 500m` and 600s proxy read/send timeouts for `/api/v1/ingest` only. The main Ingress keeps nginx's default 1 MB body limit.

Annual COTAHIST ingest is memory-heavy: the service holds the parsed rows in process memory before writing Parquet. Production requests **4Gi** and limits **8Gi**. A Cloudflare **502** during ingest with pod exit **137** / `OOMKilled` means the limit was hit, not that the ZIP was invalid. Trailer lines (`TIPOREG=99`) log a parse warning and are expected.

A `413` HTML page from nginx on that path means the request was rejected before the pod. If TLS terminates on another nginx in front of this Ingress, that hop needs `client_max_body_size 500m` as well.
