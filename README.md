# tucano-b3

API REST de alta performance para dados de mercado da B3 (Brasil, Bolsa, Balcão).

## Visão Geral

O `tucano-b3` serve dados históricos de cotações (COTAHIST) e a série histórica
do IBOVESPA via REST API. Os dados são ingeridos via [`tucano-b3-cli`](https://github.com/tucano-b3/tucano-b3-cli)
e servidos a partir de arquivos Parquet com alta performance.

## Instalação

### Via cargo
```bash
cargo install tucano-b3
```

### Via binário pré-compilado
Baixe da [página de releases](https://github.com/tucano-b3/tucano-b3/releases).

### Via Homebrew (macOS/Linux)
```bash
brew install tucano-b3
```

### Via Docker
```bash
docker pull ghcr.io/tucano-b3/tucano-b3:latest
docker run -p 8080:8080 -v ./data:/data:ro ghcr.io/tucano-b3/tucano-b3
```

## Uso Rápido

1. Ingerir dados com `tucano-b3-cli` (veja a documentação do CLI)
2. Iniciar o servidor:
```bash
tucano-b3 --config config.toml
```
3. Consultar dados:
```bash
curl http://localhost:8080/api/v1/cotahist/tickers/VALE3/latest
curl http://localhost:8080/api/v1/ibov/history?dias=5
```

## Endpoints

| Método | Rota | Descrição |
|--------|------|-----------|
| GET | `/api/v1/cotahist/tickers` | Listar tickers |
| GET | `/api/v1/cotahist/tickers/{ticker}` | Detalhe do ticker |
| GET | `/api/v1/cotahist/tickers/{ticker}/quotes` | Histórico de cotações |
| GET | `/api/v1/cotahist/tickers/{ticker}/latest` | Última cotação |
| GET | `/api/v1/cotahist/dates/{date}/quotes` | Cotações por data |
| GET | `/api/v1/ibov/history` | Série histórica IBOV |
| GET | `/api/v1/ibov/latest` | Último dado IBOV |
| GET | `/api/v1/health` | Health check |
| GET | `/metrics` | Métricas Prometheus |
| GET | `/api/docs/` | Swagger UI |

## Configuração

Veja `config.toml` para todas as opções.

## Documentação da API

Acesse `http://localhost:8080/api/docs/` para a documentação interativa (Swagger UI).

## Licença

MIT / Apache-2.0
