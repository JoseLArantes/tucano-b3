---
title: Descoberta e atualizações pendentes
---

# Descoberta e pendências

A descoberta leve consulta canais registrados e grava referências de publicação que podem exigir avaliação. Ela é distinta de baixar, inventariar, interpretar ou publicar um arquivo.

## Execução manual

POST /api/v1/b3/discovery/scan exige token administrativo e não recebe corpo. A resposta inclui checked_at em UTC e scheduled=false. O scanner usa capacidades e elegibilidade registradas; falhas de coordenação ou configuração aparecem como erro, não como sucesso vazio.

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
