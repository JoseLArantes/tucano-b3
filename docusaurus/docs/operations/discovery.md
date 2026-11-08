---
title: Descoberta e atualizações pendentes
---

# Descoberta e pendências

A descoberta leve consulta canais registrados no PostgreSQL e percorre locais adicionais definidos pelo operador. Ela registra apenas observações de metadados. Não baixa, extrai, interpreta ou publica arquivos e não inicia jobs de aquisição.

## Locais configuráveis

`B3_DISCOVERY_LOCATIONS` é uma lista separada por ponto e vírgula (`;`) ou vírgula (`,`). Cada item usa `tipo:valor`; prefira `;` se o caminho/prefixo puder conter vírgula:

```dotenv
B3_DISCOVERY_LOCATIONS=folder:/data/raw/imports;b3_url:https://www.b3.com.br/caminho/COTAHIST_A2025.ZIP;object_storage_bucket:s3://meu-bucket/cotahist/
```

| Tipo | Valor | Como é verificado | Limites e observações |
|---|---|---|---|
| `folder` | Caminho absoluto visível ao processo, por exemplo `/data/raw/imports` | Percorre arquivos e subpastas sem seguir links simbólicos; consulta nome e metadados do sistema de arquivos | Até 10.000 entradas e 8 níveis por local. No Compose, `./data/imports` é montado somente leitura em `/data/raw/imports`. Não aponte para todo `/data`, pois a pasta também pode conter arquivos do PostgreSQL. Pastas adicionais precisam ser montadas no API e no worker. |
| `b3_url` | URL HTTPS direta para um arquivo `.TXT` ou `.ZIP` com nome COTAHIST | `HEAD`, validando HTTPS, host em `B3_ALLOWED_HOSTS`, DNS e cada redirecionamento | Não aceita página de índice/HTML, query string, fragmento ou credenciais na URL. A URL deve usar o nome do artefato, por exemplo `COTAHIST_A2025.ZIP`. Não há fallback para GET. `host:` é aceito como alias compatível. |
| `object_storage_bucket` | URI `s3://bucket/prefixo/` | AWS SDK `ListObjectsV2`, apenas metadados de objetos | Até 10.000 objetos por local. Usa a cadeia padrão de credenciais AWS e exige permissão somente de listagem (`s3:ListBucket` restrita ao bucket/prefixo). Nunca executa `GetObject`. |

Locais desconhecidos, caminhos relativos, URLs fora da lista de hosts B3 ou URLs sem arquivo COTAHIST identificável falham a validação de configuração; não são ignorados. Até 32 locais podem ser configurados. Para S3 compatível, defina `AWS_REGION` (padrão `us-east-1`) e `AWS_ENDPOINT_URL_S3` com o endpoint do serviço. Configure também as credenciais AWS apropriadas ou a identidade de workload. No Compose, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` e `AWS_SESSION_TOKEN` são encaminhadas às duas aplicações quando fornecidas. Use permissão de leitura/listagem e nunca credenciais de escrita.

Os padrões de nome reconhecidos são `COTAHIST_A{AAAA}.ZIP|TXT`, `COTAHIST_M{MMAAAA}.ZIP|TXT` e `COTAHIST_D{DDMMAAAA}.ZIP|TXT`. O nome classifica a granularidade; não determina uma data de referência, validade do conteúdo ou precedência entre versões. `ETag` é preservado como metadado remoto e não é tratado como SHA-256.

## Execução manual

`POST /api/v1/b3/discovery/scan` exige o valor de `TUCANO_B3_ADMIN_TOKEN` no Bearer e não recebe corpo. Um token de leitura, inclusive um valor legado configurado como `TUCANO_B3_TOKEN`, recebe `403 ADMIN_REQUIRED`. A UI Swagger não carrega `.env`; informe o token administrativo no botão “Authorize”.

A resposta `DiscoveryResult` contém `checked_at` UTC, `scheduled`, totais, resultado por local e até 1.000 candidatos. Cada local retorna `kind`, rótulo posicional (`folder-1`, `b3_url-2`), `outcome` (`available`, `absent`, `inconclusive` ou `failed`), quantidade e indicador de truncamento. Cada candidato retorna `source_file_type`, `source_locator`, `byte_size`, `remote_etag` e `remote_last_modified`. O locador de pasta é relativo e rotulado; não inclui o caminho raiz configurado.

Se a configuração tiver mais candidatos que o limite de resposta, `candidates_truncated=true`; se um limite por local for atingido, o próprio local também marca `truncated=true`. Falha em um diretório, URL ou bucket aparece como `failed` e não impede tentativas nos outros locais. Falha ao persistir a observação, ao contrário, falha a chamada. As checagens e seus candidatos ficam em `b3_discovery_location_checks` e `b3_discovery_location_candidates`.

Um candidato encontrado em pasta, URL ou bucket **não** cria uma pendência de canal nem comprova elegibilidade, autorização de coleta, retenção ou redistribuição. O operador ainda precisa registrar e homologar um canal oficial apropriado antes de submetê-lo a ingestão. O scanner não calcula hash de conteúdo e não transforma observação de localização em arquivo bruto.

Exemplo de resposta:

```json
{
  "checked_at": "2026-11-08T15:00:00Z",
  "scheduled": false,
  "locations_scanned": 1,
  "candidate_count": 1,
  "candidates_truncated": false,
  "locations": [
    { "kind": "folder", "label": "folder-1", "outcome": "available", "candidate_count": 1, "truncated": false }
  ],
  "candidates": [
    {
      "source_file_type": "COTAHIST_YEARLY",
      "source_locator": "folder:1/2025/COTAHIST_A2025.ZIP",
      "byte_size": 12345,
      "remote_etag": null,
      "remote_last_modified": "2026-11-08T14:45:00+00:00"
    }
  ]
}
```

O scan manual não renova o indicador de descoberta agendada e não significa que um novo fato de mercado tenha sido persistido.

## Lista de pendências

GET /api/v1/b3/pending-updates?limit=100&offset=0 lista pendências por detecção e UUID. A resposta é paginada e inclui id, channel_id, source_file_type, reference_date quando única, status e detected_at. O endpoint exige token administrativo e não inicia download.

## Reconhecimento administrativo

POST /api/v1/b3/pending-updates/&#123;id&#125;/acknowledge recebe JSON fechado:

```json
{
  "source_file_id": "UUID_DO_ARQUIVO",
  "reason": "Publicação já corresponde ao arquivo preservado."
}
```

reason é obrigatório, não pode ser vazio e é limitado a 1024 bytes. O arquivo deve sustentar a publicação no mesmo canal. A resposta retorna id e source_file_id. Reconhecer registra uma decisão; não inventa contadores, não muda preço e não dispensa revalidação periódica.

Erros comuns incluem 400 para corpo/motivo inválido, 403 sem administração, 404 para pendência/arquivo ausente e 409 para canal ou evidência incompatível.
