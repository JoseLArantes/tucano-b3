---
title: Instalação e execução local
---

# Instalação e execução local

O serviço e o site de documentação são processos separados. A API e o worker usam PostgreSQL; o Docusaurus apenas compila páginas estáticas e não precisa de uma base de dados.

## Requisitos

- Docker com Compose para executar a stack local do serviço;
- Node.js 20 ou superior e npm para executar ou compilar o site;
- variáveis locais preenchidas a partir de .env.example.

## API e worker

Na raiz do repositório, prepare as variáveis e defina credenciais Bearer de leitura e administração distintas e não vazias. Mantenha a credencial administrativa privada. Em seguida, use `docker compose up -d`:

    cp .env.example .env
    docker compose config --quiet
    docker compose up -d
    docker compose ps --all

O Compose espera PostgreSQL saudável, executa o serviço `b3_migrate` usando `B3_MIGRATION_DATABASE_URL` e só inicia API/worker depois que as migrations terminam com sucesso. API, worker e MCP verificam o schema; seus processos não executam DDL. Para reaplicar a cadeia manualmente, rode `docker compose run --rm b3_migrate`; para reconstruir imagens após alterações de código, use `docker compose up --build -d`.

O Compose local publica a API na porta 8010 do host e PostgreSQL na 5433. Na rede interna Compose, a API usa a porta 8000 e PostgreSQL usa 5432. As credenciais do exemplo são somente locais; substitua-as fora do ambiente de desenvolvimento. O serviço `cargo` pertence ao profile `tools` e não inicia com a stack de runtime.

Para confirmar que o processo HTTP responde sem autenticação, use a checagem de liveness:

    curl http://localhost:8010/healthz

Essa resposta não verifica cobertura de dados. GET /api/v1/health verifica a dependência PostgreSQL e também é uma exceção pública.

## Site Docusaurus

Na raiz do projeto, instale as dependências com o lockfile e inicie o site:

    npm --prefix docusaurus ci
    npm --prefix docusaurus start

Abra http://localhost:3000. O site não chama a API automaticamente. Exemplos de integração nas páginas seguintes usam a API em http://localhost:8010.

## Build estático

Compile e sirva a saída localmente para revisar os arquivos publicados:

    npm --prefix docusaurus run build
    npm --prefix docusaurus run serve

O build fica em docusaurus/build/. Links internos inválidos fazem o build falhar.
