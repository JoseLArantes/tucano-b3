---
title: Operação da ingestão
---

# Operação do serviço

A operação separa descoberta de publicações, aquisição e processamento pesado, revisão e publicação. A API aceita comandos administrativos duráveis; o worker realiza o trabalho fora da requisição HTTP e grava contadores e estados confirmados no PostgreSQL.

## Princípios

- A descoberta leve registra publicações potenciais sem baixar datasets.
- Jobs são duráveis e compartilhados por API, CLI e worker.
- Prioridade de manutenção mantém capacidade reservada para atualizações regulares.
- Cancelamento é registrado como pedido durável e aplicado nos limites seguros do executor.
- Arquivos brutos e inventário de members são preservados para inspeção e reprocessamento.
- Uma revisão publicada não é trocada por candidato incompleto.
- Erro de infraestrutura ou de arquivo permanece visível; não vira uma resposta vazia bem-sucedida.

## Fluxo geral

    descoberta → pendência → comando administrativo → aquisição/inventário → parsing em lotes → revisão pronta → publicação explícita

O fluxo pode terminar com falha, issues ou cancelamento. A consulta de dados não inicia esse fluxo.

## Segurança e autorização

POST de comando, cancelamento, retry, scan e reconhecimento de pendência exige Bearer administrativo. GET de jobs, sync-runs, arquivos, members, pendências e revisões também exige administração. Não publique credenciais administrativas a consumidores da API.

## Antes de habilitar fonte

O endpoint de capacidades informa famílias implementadas, adiadas e elegibilidade para REST e entrega a terceiros. Um canal configurado não basta: a finalidade precisa ter evidência válida de elegibilidade. Nenhuma fonte é habilitada por seed. Para procedimentos de runtime e retenção, consulte também [release_operations.md](https://github.com/JoseLArantes/b3-tucano/blob/main/docs/release_operations.md).
