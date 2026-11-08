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

    descoberta → pendência → comando administrativo → aquisição/inventário → parsing em lotes → gates de qualidade → publicação automática da revisão limpa

Uma revisão com rejeições ou conflitos não é publicada automaticamente e termina com issues para decisão administrativa. O fluxo também pode terminar com falha ou cancelamento. A consulta de dados não inicia esse fluxo.

## Cache de exportação COTAHIST

`GET /api/v1/b3/cotahist/quotes/export` usa o PostgreSQL para gerar lotes keyset de uma revisão publicada e grava uma cópia derivada em `B3_PROCESSED_DATA_DIR/quote-exports`. O manifesto PostgreSQL coordena uma geração por chave, registra lease/checksum/tamanho e aplica expiração de 30 dias. A perda de um arquivo invalida a entrada e permite reconstruí-la; não remova o schema ou a revisão para limpar a cache.

O volume de exportações não contém fonte oficial nem é necessário para backup/restore do histórico. A aplicação pode montá-lo em storage persistente compartilhado quando tiver várias instâncias no mesmo deployment; a configuração Compose local não oferece filesystem multi-host ou object storage. Autorização e elegibilidade externa são verificadas antes de usar cada artefato. Como o endpoint é finito, o evento `complete` valida término; uma conexão encerrada antes dele é incompleta.

## Segurança e autorização

POST de comando, cancelamento, retry, scan e reconhecimento de pendência exige Bearer administrativo. GET de jobs, sync-runs, arquivos, members, pendências e revisões também exige administração. Não publique credenciais administrativas a consumidores da API.

## Antes de habilitar fonte

O endpoint de capacidades informa famílias implementadas, adiadas e elegibilidade para REST e entrega a terceiros. Um canal configurado não basta: a finalidade precisa ter evidência válida de elegibilidade. Nenhuma fonte é habilitada por seed. Para procedimentos de runtime e retenção, consulte também [release_operations.md](https://github.com/JoseLArantes/b3-tucano/blob/main/docs/release_operations.md).
