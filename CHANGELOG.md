# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0](https://github.com/JoseLArantes/tucano-b3/compare/v0.1.0...v1.0.0) (2026-11-13)


### Features

* add revision-pinned ticker quote export ([5f78713](https://github.com/JoseLArantes/tucano-b3/commit/5f78713ad71d437723380e0394c7e0386d87ec62))
* **api:** add OpenAPI spec builder and Swagger UI endpoint annotations (M6.1-M6.6) ([4a6a114](https://github.com/JoseLArantes/tucano-b3/commit/4a6a1147351e5157e2ee34c92d417281a918344b))
* **api:** add Prometheus /metrics endpoint and integration contract tests ([cca1459](https://github.com/JoseLArantes/tucano-b3/commit/cca1459f91a0a23cb279f38cc9cc563ba7157b1a))
* **api:** add upload and folder ingestion REST endpoints (M10.5) ([5efb9b8](https://github.com/JoseLArantes/tucano-b3/commit/5efb9b8a0c1a00b56bdf7b08cd533dc439c42c28))
* **api:** implement REST API, OpenAPI 3.1, background worker, and MCP server (Phase 2) ([7b5f3f7](https://github.com/JoseLArantes/tucano-b3/commit/7b5f3f733d7fe45c7697bda7ebc310094a5a0a8a))
* **auth:** require token from TUCANO_B3_TOKEN on all endpoints except health and add redoc ([96f4f71](https://github.com/JoseLArantes/tucano-b3/commit/96f4f712f7e5fccdf8d7877d1e028dbc9f0ec94b))
* **cache:** implement in-memory cache store, key generation, and middleware (M4.1-M4.6) ([9644efe](https://github.com/JoseLArantes/tucano-b3/commit/9644efe4ca671956750add37ce40a08f93cc54a1))
* configure B3 discovery and update Swagger ([4c651c2](https://github.com/JoseLArantes/tucano-b3/commit/4c651c2385ae97cc6dd46de3c63d34ed671646a9))
* **coordination:** enhance advisory locks, read-only MCP startup, and timezone boundary testing (R4/R5) ([aeacc8f](https://github.com/JoseLArantes/tucano-b3/commit/aeacc8f404c7c607f5fbb4fce22e390da7d0ff92))
* **core:** complete Phase 3 hardening, edge-case test suites, and documentation ([cae570a](https://github.com/JoseLArantes/tucano-b3/commit/cae570a7b5372681adfcda062361b9dd32d4502d))
* **cotahist:** add analytics endpoints and refresh public docs/ ([4732700](https://github.com/JoseLArantes/tucano-b3/commit/4732700cc184e82b3cf5cdef169ed5a814bf3c72))
* **docker:** configure Dockerfile, Dockerfile.dev and docker-compose for development workflow ([93b60f2](https://github.com/JoseLArantes/tucano-b3/commit/93b60f2b8e0768682fc944da273d253775fbf360))
* docs phase - def milestone 0 ([cab2222](https://github.com/JoseLArantes/tucano-b3/commit/cab2222ec60f1952c242966f4ecc8f88e768c61b))
* docs phase - def milestone 0 and 1 ([56e7a78](https://github.com/JoseLArantes/tucano-b3/commit/56e7a781170e3a21f404b4b1d3b6fb09795e519e))
* docs phase - def milestone 2 ([7e50120](https://github.com/JoseLArantes/tucano-b3/commit/7e5012038626764c3e4f10dced5523876b4d27f3))
* docs phase - def milestone 3 ([ee5034b](https://github.com/JoseLArantes/tucano-b3/commit/ee5034b7d9cf05343aff0b72c39d6d757844b3c8))
* docs phase - def milestone 4 and 5 ([f4c8a3e](https://github.com/JoseLArantes/tucano-b3/commit/f4c8a3ea80661818f84d961259ce78594edcb96c))
* docs phase - def milestone 6 ([b151ddb](https://github.com/JoseLArantes/tucano-b3/commit/b151ddb8edb93fc6aea10c68f92b9c3d2c05bd50))
* docs phase - def milestone 7 ([8a17259](https://github.com/JoseLArantes/tucano-b3/commit/8a1725924bceea221b3dec5bf83331805f0682e7))
* docs phase - def milestone 8 ([cb00854](https://github.com/JoseLArantes/tucano-b3/commit/cb00854dd58e4b8dd911f79c276b684cca2b068f))
* docs phase - def milestone 9 ([a69ea40](https://github.com/JoseLArantes/tucano-b3/commit/a69ea407d3e0d5b4c1e2a43852bfaa8d9b704d49))
* docs phase 0 ([8ae445f](https://github.com/JoseLArantes/tucano-b3/commit/8ae445f2607f32101e4054139cefed7c1072597d))
* docs phase 0 ([319db8e](https://github.com/JoseLArantes/tucano-b3/commit/319db8e4836417ab07a53c59e9f7435c3d7a90b8))
* docs phase 0 - docs ([383b7ac](https://github.com/JoseLArantes/tucano-b3/commit/383b7ac61e355565413f2fc7a109c69a68e019d3))
* **docs:** add interactive Swagger UI and ReDoc with Bearer authentication support ([b1b6cdf](https://github.com/JoseLArantes/tucano-b3/commit/b1b6cdfdef3526c6bf5d9bef0c3065234a6399ae))
* **ingest:** add ZIP decompression and folder scanner (M10.3) ([fd1bc6c](https://github.com/JoseLArantes/tucano-b3/commit/fd1bc6cfac1d34d2b3df8841eaf0e1468304b02d))
* **ingest:** implement COTAHIST fixed-width parser (M10.1) ([812ebef](https://github.com/JoseLArantes/tucano-b3/commit/812ebef971357b2d7d91218bab9a6839ba92cd2c))
* **ingest:** implement IngestService orchestration and incremental logic (M10.4) ([713276f](https://github.com/JoseLArantes/tucano-b3/commit/713276f64a8eb11d380ad4260afc7e739f2eff9c))
* **ingest:** implement Parquet writer for partitioned COTAHIST datasets (M10.2) ([919e042](https://github.com/JoseLArantes/tucano-b3/commit/919e04232df26d022732edf6912859417953d417))
* **ledger:** extend SQLite ledger for date tracking and ticker upserts (M10.7) ([a316790](https://github.com/JoseLArantes/tucano-b3/commit/a316790e636e2f94ae20096aa5ba5b10a23c7f55))
* **m0:** add docker and docker-compose deployment setup (M0.8) ([d06406d](https://github.com/JoseLArantes/tucano-b3/commit/d06406d793a07fb6e40a8783b74f8c4848f60f84))
* **m0:** configure structured json and pretty logging (M0.7) ([cd79bf2](https://github.com/JoseLArantes/tucano-b3/commit/cd79bf245fc0930d57caa1e989d388225cff011d))
* **m0:** implement axum server and graceful shutdown (M0.4) ([35ea7e9](https://github.com/JoseLArantes/tucano-b3/commit/35ea7e9cd4d42c1420e0999eed747a54ea050344))
* **m0:** implement configuration system and tests (M0.2) ([f3a8681](https://github.com/JoseLArantes/tucano-b3/commit/f3a86811068dd4d09296be2bc273dd9d8f658ad4))
* **m0:** implement error handling framework with pt-BR messages (M0.3) ([ba58c4b](https://github.com/JoseLArantes/tucano-b3/commit/ba58c4bf69df3735ade79504b4e79cb6d779a644))
* **m0:** implement health and info endpoints (M0.6) ([71d06ef](https://github.com/JoseLArantes/tucano-b3/commit/71d06ef940dac6e794e539c3ba3f02457f38ee05))
* **m0:** implement request id middleware (M0.5) ([a929adb](https://github.com/JoseLArantes/tucano-b3/commit/a929adb310e091bb1e8bcc144f130a5ea0452985))
* **m0:** initialize project dependencies and module structure (M0.1) ([415a308](https://github.com/JoseLArantes/tucano-b3/commit/415a308633c446cee807e1cc3d4d79a7b60bdd87))
* **m1:** add test fixtures and data layer integration tests (M1.7 - M1.8) ([52612f2](https://github.com/JoseLArantes/tucano-b3/commit/52612f245518625776cb9d6f04d7c947223a16ff))
* **m1:** extend AppState with DataLoader and update health endpoint (M1.6) ([0195286](https://github.com/JoseLArantes/tucano-b3/commit/01952869be7667146e0fc2e0841a49a3a6bfc024))
* **m1:** implement data reload mechanism and data loader (M1.5) ([ca31f38](https://github.com/JoseLArantes/tucano-b3/commit/ca31f3897e53657e7c6c444bc297b7fe6f40168d))
* **m1:** implement DataFusion session context and query service (M1.3) ([e9cd3f9](https://github.com/JoseLArantes/tucano-b3/commit/e9cd3f9eb5169fe9734f95ede67b8f8ddbda7252))
* **m1:** implement parquet file reader and discovery (M1.2) ([3cdb106](https://github.com/JoseLArantes/tucano-b3/commit/3cdb106aaf977cf2eae9fe3d79020d779013c158))
* **m1:** implement parquet schema definitions and validation (M1.1) ([ee7166b](https://github.com/JoseLArantes/tucano-b3/commit/ee7166b0f40a714e22e320f5e800916c2392597c))
* **m1:** implement ticker registry from SQLite ledger (M1.4) ([29290d1](https://github.com/JoseLArantes/tucano-b3/commit/29290d1c9759e5fe1f82bce00cd900a8e03c66e9))
* **m2:** add shared models, pagination, and response envelope (M2.1) ([3816065](https://github.com/JoseLArantes/tucano-b3/commit/3816065aa8471064f75d52bd6836d21aec5de843))
* **m2:** implement COTAHIST service layer (M2.2) ([f632bea](https://github.com/JoseLArantes/tucano-b3/commit/f632bea2a47be0e910855c471362ac132ec74aff))
* **m2:** implement date quotes endpoint (M2.6) ([480cfc8](https://github.com/JoseLArantes/tucano-b3/commit/480cfc8f34a30319791306eda1ddc5c6926aeed0))
* **m2:** implement latest quote endpoint (M2.5) ([6d822b9](https://github.com/JoseLArantes/tucano-b3/commit/6d822b93115a7db76ddaea00241af3d25220579e))
* **m2:** implement quote history endpoint (M2.4) ([edc772d](https://github.com/JoseLArantes/tucano-b3/commit/edc772d71f67fb65443e58599de0e6ce200815fc))
* **m2:** implement ticker list and detail endpoints (M2.3) ([83923d9](https://github.com/JoseLArantes/tucano-b3/commit/83923d97b03a97b8eed461d39f2339b122533e30))
* **m3:** implement IBOV data models (M3.1) ([e3c672d](https://github.com/JoseLArantes/tucano-b3/commit/e3c672d319c3c851cdc50ea193eaae5213c5909d))
* **m3:** implement IBOV history endpoint (M3.3) ([08b8337](https://github.com/JoseLArantes/tucano-b3/commit/08b8337d2b40ede4dfc08fabdca063b696845745))
* **m3:** implement IBOV latest endpoint (M3.4) ([84a8f32](https://github.com/JoseLArantes/tucano-b3/commit/84a8f3223a7f05925e836bacc39207fbb715aa4e))
* **m3:** implement IBOV service layer (M3.2) ([ff8d9ce](https://github.com/JoseLArantes/tucano-b3/commit/ff8d9cecb5ee5d98ee900c44e8a46491833e2ed3))
* **m3:** wire IBOV routes into API (M3.5) ([e269fe7](https://github.com/JoseLArantes/tucano-b3/commit/e269fe797c501d0d2543e51b30f90af0eaeba7d1))
* **m5:** add API key authentication and fallback tests (M5.5) ([78a3ef3](https://github.com/JoseLArantes/tucano-b3/commit/78a3ef39a11892471daca0d87f53822e14080836))
* **m5:** implement API key registry (M5.2) ([ad94ca1](https://github.com/JoseLArantes/tucano-b3/commit/ad94ca19daccd6f3ccac8dac44e7f8658261b0f7))
* **m5:** implement rate limit state and tower layer (M5.3) ([5fb226a](https://github.com/JoseLArantes/tucano-b3/commit/5fb226a03891f28fb1d4bc036768bdbcd34a0233))
* **m5:** implement token bucket rate limiter (M5.1) ([d5d5525](https://github.com/JoseLArantes/tucano-b3/commit/d5d55251ddeaee0e80650ab55bc6d996a057a73b))
* **m5:** wire rate limiter into server and main (M5.4) ([091c2ab](https://github.com/JoseLArantes/tucano-b3/commit/091c2abbac5c929e691a37f10fd5cefab7efee10))
* **observability:** implement Prometheus metrics registry, metrics endpoint, and middleware (M7.1-M7.6) ([3afcf97](https://github.com/JoseLArantes/tucano-b3/commit/3afcf97c190ad70b969aec1e1c7fdce13f57b0ae))
* rebuild tucano-b3 in Rust (Phase 1 foundation) ([3fd0535](https://github.com/JoseLArantes/tucano-b3/commit/3fd0535532a65480febb8fcd2c2d09964d35b8f5))
* refactoring to 0 ([d531a51](https://github.com/JoseLArantes/tucano-b3/commit/d531a51f08bfc144c096f56d8cb74b8958d51666))
* **release:** add durable job and source eligibility foundations ([f64fdf5](https://github.com/JoseLArantes/tucano-b3/commit/f64fdf5e6a7322a006971d83c58a4d497d32e6d3))
* **release:** integrate historical 0.0.1 pipeline ([7e93b0d](https://github.com/JoseLArantes/tucano-b3/commit/7e93b0d3f16662502ad43b142b8a6ec6819bdec9))
* **release:** qualify 0.0.1 historical platform ([f672eec](https://github.com/JoseLArantes/tucano-b3/commit/f672eec87b476a28bd9b9bc2d552dbdfbb186d2c))
* support the use of object storage as ingestion source ([7edb007](https://github.com/JoseLArantes/tucano-b3/commit/7edb00722809c8615c437b3fe05095a972956891))


### Bug Fixes

* allow bounded history candidate carry-forward ([f7b74d2](https://github.com/JoseLArantes/tucano-b3/commit/f7b74d2a157289a546ce1e3ce77a80ccb55a0b6f))
* **assessment:** correct sync lock retention diagnosis and reclassify mcp startup evidence ([1eb6765](https://github.com/JoseLArantes/tucano-b3/commit/1eb676559018c346097e3fdafe55831abbd4a301))
* bound history carry-forward and normalize COTAHIST text encoding ([a6a09ce](https://github.com/JoseLArantes/tucano-b3/commit/a6a09ceb381105bc013927fb94ebbccaeef30df3))
* **ci:** ensure data directory exists during test execution ([c734dde](https://github.com/JoseLArantes/tucano-b3/commit/c734dde1e39670c713def0071ce87ada3d1ba4ed))
* **contracts:** correct health routes, env token name, and index table schemas in inventory ([f5be6df](https://github.com/JoseLArantes/tucano-b3/commit/f5be6df69439314d67b46a6f343b1cdc4524765c))
* docker configs ([d6fe81e](https://github.com/JoseLArantes/tucano-b3/commit/d6fe81e3869d4b0ed9bb4c7ede72d4f52f3e9561))
* docker configs ([b42eb7f](https://github.com/JoseLArantes/tucano-b3/commit/b42eb7fc10b7be1e87e8fc44adda5b8ccadce40a))
* **docker:** include benches in build context and stub data_layer bench in Dockerfile ([12b30b2](https://github.com/JoseLArantes/tucano-b3/commit/12b30b25e4f4e71ad37afb0c9215b1dca8229407))
* **docker:** support pre-built binaries, add lld linker, and fix benches copy ([2b65c9b](https://github.com/JoseLArantes/tucano-b3/commit/2b65c9b84de2d09ef6738af2dc580a2240bb80e3))
* **openapi:** enable BearerAuth and ApiKeyAuth in Swagger UI with persisted authorization ([6068cb0](https://github.com/JoseLArantes/tucano-b3/commit/6068cb02172097eecc821e1f8b0062258d123b63))
* **openapi:** resolve generic T schema pointer in ApiResponse for Swagger UI ([bb41e27](https://github.com/JoseLArantes/tucano-b3/commit/bb41e27a5ec476ce3e315eaf3895506fe5355d77))
* **parsers:** reject over-depth ZIPs visibly, add representative nested ISO 20022 fixtures, and remove unapproved query scope ([02c4ed1](https://github.com/JoseLArantes/tucano-b3/commit/02c4ed137fdc0a4bc5c3c2a213b3c46e595d3723))
* skip ci ([01e6271](https://github.com/JoseLArantes/tucano-b3/commit/01e627152b34782e28a8957b63d46527f2626716))
* support local COTAHIST development imports ([6b0922a](https://github.com/JoseLArantes/tucano-b3/commit/6b0922add0a3835b494fd967f3f861f5a788f37f))
* tests ([6f48beb](https://github.com/JoseLArantes/tucano-b3/commit/6f48beb4461c67902a980a0ddcdbade4d2145f2d))
* validate COTAHIST trailer detail counts ([492b26e](https://github.com/JoseLArantes/tucano-b3/commit/492b26ea661d7fb8b7969239f599e998d331af87))


### Performance Improvements

* accumulate history counters during batch writes ([643b71f](https://github.com/JoseLArantes/tucano-b3/commit/643b71f17029421efeaaf81be74eea5a5409f853))
* **benches:** add comprehensive API performance benchmark harness (M9.1) ([ba5e84a](https://github.com/JoseLArantes/tucano-b3/commit/ba5e84a6dfa23899332109953ca92df81f1e924b))
* bound duplicate checks to indexed revisions ([5bae61f](https://github.com/JoseLArantes/tucano-b3/commit/5bae61fabd9fb45fc6a1a64e0ae677f4177a47f4))
* **cache:** enforce 1MB body size cap to prevent memory bloat and add memory profiling script (M9.4) ([91620be](https://github.com/JoseLArantes/tucano-b3/commit/91620be59bae1782642ea1949824fff2b2ac7d97))
* index staged quote revision lookups ([bbbb1bc](https://github.com/JoseLArantes/tucano-b3/commit/bbbb1bc6a03ec37622db3cf086729f19338442a1))
* **load-test:** add load test scripts for wrk and vegeta with docker fallback (M9.2) ([e92e300](https://github.com/JoseLArantes/tucano-b3/commit/e92e300bf7c6ea311c6fe0923c44a1f896ef3a78))
* **query:** optimize DataFusion session config and add query optimizer helpers (M9.3) ([3298ba7](https://github.com/JoseLArantes/tucano-b3/commit/3298ba7d545e37207d2ab2230cdf1c300a7eeb7e))
* **serialization:** optimize response metadata timestamps and cache body forwarding (M9.5) ([c13e89c](https://github.com/JoseLArantes/tucano-b3/commit/c13e89c638fb2994d6b79034b166b30676643736))


### Documentation

* add comprehensive README in pt-BR and ARCHITECTURE.md with Mermaid diagram ([043fc1b](https://github.com/JoseLArantes/tucano-b3/commit/043fc1b83263e7c8ed9937eefaff21184a59b3a8))
* add Docusaurus site ([1ce6b33](https://github.com/JoseLArantes/tucano-b3/commit/1ce6b339a7f1faab868aab35c215a729b9435395))
* add MIT license ([5bec964](https://github.com/JoseLArantes/tucano-b3/commit/5bec964436ef1c9fc9b42e6cc770f5cadd6d4ece))
* **api:** add comprehensive OpenAPI 3.1 documentation for all endpoints and schemas ([1e7ec9d](https://github.com/JoseLArantes/tucano-b3/commit/1e7ec9de97466376e85f37cdfad6a0615ed80641))
* consumers updates ([8a4b435](https://github.com/JoseLArantes/tucano-b3/commit/8a4b4351486804eb64e34ca8c3c2e98c3a95b82e))
* **extension:** add frontend and consumer endpoints plan and update docs deployment URL ([813af8d](https://github.com/JoseLArantes/tucano-b3/commit/813af8dce459b5fd8ccd5a98577cc52a6fcc08b9))
* **m0:** complete verification checklist for milestone M0 (M0.10) ([ef102af](https://github.com/JoseLArantes/tucano-b3/commit/ef102afdcbe55ef9ed6b26494967166951977c29))
* **m1:** complete verification checklist for milestone M1 (M1.10) ([c163095](https://github.com/JoseLArantes/tucano-b3/commit/c1630952cbe2922bc3c3051278847be85c2e2102))
* **m2:** complete verification checklist for milestone M2 (M2.9) ([b13f534](https://github.com/JoseLArantes/tucano-b3/commit/b13f534ff5722bc5461894571446404a11546b7b))
* **m3:** complete verification checklist for milestone M3 (M3.7) ([f642d88](https://github.com/JoseLArantes/tucano-b3/commit/f642d880c5ac32344a3434518026f4c693f4a5ec))
* **m5:** complete verification checklist for milestone M5 (M5.7) ([73b695a](https://github.com/JoseLArantes/tucano-b3/commit/73b695a8bc12b853915770ad3418f253782b771f))
* **migration:** define complete-file publication scope and active-revision pointer filtering ([a95b5a1](https://github.com/JoseLArantes/tucano-b3/commit/a95b5a1b8df2fe39c8df0601c5aca4afd8c0a0d2))
* **migration:** define explicit active revision pointer, versioned facts, and correct identifier window constraint ([68607a1](https://github.com/JoseLArantes/tucano-b3/commit/68607a1be89c7eea08c18ed4a1bb40aa6a38d3d9))
* **migration:** define revision-scoped uniqueness, public_id preservation, and reconstructive membership validation ([fdc804f](https://github.com/JoseLArantes/tucano-b3/commit/fdc804fd84ca286fb2c26057182808a61cf36b50))
* **milestones:** complete verification checklist for milestone M10 (M10.10) ([2fe31c3](https://github.com/JoseLArantes/tucano-b3/commit/2fe31c37d5eb28e8859c12b545202315d855c43c))
* **milestones:** complete verification checklist for milestone M4 (M4.8) ([14d8d39](https://github.com/JoseLArantes/tucano-b3/commit/14d8d3944f9a5855a865d82506d287ab3c1d8d0f))
* **milestones:** complete verification checklist for milestone M6 (M6.8) ([b2bd8b0](https://github.com/JoseLArantes/tucano-b3/commit/b2bd8b0b04e0e0d11e1c9af4eddef9decac6f1b0))
* **milestones:** complete verification checklist for milestone M7 (M7.8) ([1030d25](https://github.com/JoseLArantes/tucano-b3/commit/1030d2568f9c84b7299a86ddf49ca8902c501f97))
* **milestones:** complete verification checklist for milestone M8 (M8.8) ([76732b8](https://github.com/JoseLArantes/tucano-b3/commit/76732b8c5014892ffe3d3f170d96327f10a8b15e))
* **milestones:** complete verification checklist for milestone M9 (M9.8) ([3cf5682](https://github.com/JoseLArantes/tucano-b3/commit/3cf56823a1a3d6aa62dd05405722bfcefaf958e6))
* **openapi:** register ingest endpoints and upload schemas in OpenAPI (M10.6) ([d08feea](https://github.com/JoseLArantes/tucano-b3/commit/d08feea14101d429e2719037488d6b7868b4c13b))
* **perf:** add comprehensive performance guide and tuning documentation (M9.7) ([f1bebdb](https://github.com/JoseLArantes/tucano-b3/commit/f1bebdb1f9c9297b6a3d12588b300f6562e6b792))
* **release:** align milestones with fresh 0.0.1 launch ([c58a40b](https://github.com/JoseLArantes/tucano-b3/commit/c58a40befa9ee9424290c2ceaf79ed407931951d))
* **release:** finalize README, developer guide, changelog, and architecture doc (M8.7) ([8e6d80a](https://github.com/JoseLArantes/tucano-b3/commit/8e6d80a7cc1837568430125797080c6802c44b3f))
* **review:** align contracts inventory routes and envelope, additive calendar migration, and atomic transaction boundaries ([077e8e1](https://github.com/JoseLArantes/tucano-b3/commit/077e8e1313346b92663bfb193f4e0e455d9ed147))
* **review:** finalize R6 assessment matrix and remediation backlog handoff ([a2d6e88](https://github.com/JoseLArantes/tucano-b3/commit/a2d6e88345b1139efb60f4e219c59d0f80bd83bf))
* **review:** replace reconstructed test output with verbatim execution transcript and correct release gate blockers ([32f6281](https://github.com/JoseLArantes/tucano-b3/commit/32f62819dae1aa564c6312e49ea317086cee7bc1))
* update architecture document ([d5dfc5d](https://github.com/JoseLArantes/tucano-b3/commit/d5dfc5d19819587cfbe4252f99e9c6441547d0c4))


### Styles

* format api_keys.toml with final newline ([de39e66](https://github.com/JoseLArantes/tucano-b3/commit/de39e6685489fb1639df603e0fee0cef8f1dddee))
* format trailing newlines in documentation and configuration files ([8777fdf](https://github.com/JoseLArantes/tucano-b3/commit/8777fdf903d0290df3a69af0a6d1b3acfd743396))


### Miscellaneous Chores

* adjust k8s limits and secret ([9dcf946](https://github.com/JoseLArantes/tucano-b3/commit/9dcf946cf9dfea1ba795feed4e97fb5f6bac1ef5))
* **agents:** update delivery checklist for conventional commits by tasks ([0cbbf47](https://github.com/JoseLArantes/tucano-b3/commit/0cbbf47ee2575d5e780954607c3d7e48ff99079b))
* Chunking the Parquet write ([6995b97](https://github.com/JoseLArantes/tucano-b3/commit/6995b978330697a87557b59b574f753ea232ff86))
* improve ci stages ([29ee071](https://github.com/JoseLArantes/tucano-b3/commit/29ee071569d21014c7217114e0bba6cc8dcc01df))
* improve documents ([5329d54](https://github.com/JoseLArantes/tucano-b3/commit/5329d54c0684e77d8722622370830e0bb7d57ac5))
* improve documents ([0038414](https://github.com/JoseLArantes/tucano-b3/commit/003841451c59dbc2e3ec2d908e56b1f4790811a7))
* improve documents ([63e778e](https://github.com/JoseLArantes/tucano-b3/commit/63e778e68a7ae92cb9ee587109d6011fe2823088))
* improve ingestion ([d3a8905](https://github.com/JoseLArantes/tucano-b3/commit/d3a89054e2de0cefc94ebd7fc191cd72ce75bbe3))
* improve ingestion process ([332a0d1](https://github.com/JoseLArantes/tucano-b3/commit/332a0d166f57be827d11fe7ec628039fd9122447))
* improvements ([e797143](https://github.com/JoseLArantes/tucano-b3/commit/e79714390e7dc455f862883026cfea95476def43))
* improvements ([c60b421](https://github.com/JoseLArantes/tucano-b3/commit/c60b42108be8290a11d540e9eb073129b03d6ab9))
* increase upload limit to up to 500MB on k8s ([4a55d04](https://github.com/JoseLArantes/tucano-b3/commit/4a55d049d61a33d249a4ee2a95dbc9b7bd7f7c22))
* **packaging:** add Homebrew formula for macOS and Linux (M8.5) ([d8dc232](https://github.com/JoseLArantes/tucano-b3/commit/d8dc23219be0da99b5371ea8515552b3bb4062b6))
* **policy:** add cargo-deny configuration (M8.6) ([2b82c75](https://github.com/JoseLArantes/tucano-b3/commit/2b82c754c8f6f88a028c4ba638c329f76584cc5e))
* **release:** configure release profile, package metadata, and CLI help/version flags (M8.1) ([eb1a318](https://github.com/JoseLArantes/tucano-b3/commit/eb1a31850495a27bfb1e0cb77419ff180476780c))
* turn on ci ([97cba57](https://github.com/JoseLArantes/tucano-b3/commit/97cba575494fdc5f38a5955d8a754ce2561eb583))


### Tests

* **api:** add OpenAPI spec and Swagger UI integration tests (M6.7) ([4085fa5](https://github.com/JoseLArantes/tucano-b3/commit/4085fa515c3b1277c7e160afbc333ef67d4b9084))
* **cache:** add unit and integration tests for caching (M4.7) ([068cd0e](https://github.com/JoseLArantes/tucano-b3/commit/068cd0e0216d653caada8b989813abb2806fcf8d))
* **ingest:** add integration and end-to-end ingestion tests (M10.8, M10.9) ([b2b9403](https://github.com/JoseLArantes/tucano-b3/commit/b2b9403a6a5a718929a14d0fd9dcfa87bccd799d))
* **integration:** harden integration tests and capture R0/R1 review evidence ([466ccee](https://github.com/JoseLArantes/tucano-b3/commit/466cceee1f1a935b02e538553b6346e0df37cf12))
* **m0:** add e2e integration tests for health and info endpoints (M0.9) ([eb6c810](https://github.com/JoseLArantes/tucano-b3/commit/eb6c810ae1f2e79f696700b973cc26866d319c44))
* **m1:** add performance benchmarks for data layer (M1.9) ([6d4cc02](https://github.com/JoseLArantes/tucano-b3/commit/6d4cc021dd8c01a81e4b49cd1bdcb707489500e0))
* **m2:** add COTAHIST integration tests (M2.8) ([b3ad294](https://github.com/JoseLArantes/tucano-b3/commit/b3ad2946bf1e24a6191c91ba075dfef931081b65))
* **m2:** verify COTAHIST route registrations (M2.7) ([c59c2d7](https://github.com/JoseLArantes/tucano-b3/commit/c59c2d7b57a9e63637821e156edce17c724072d3))
* **m3:** add IBOV integration tests (M3.6) ([e38db02](https://github.com/JoseLArantes/tucano-b3/commit/e38db02245dec990c9302fdb24a25a0cd33ba861))
* **m5:** add rate limit integration tests (M5.6) ([6268c22](https://github.com/JoseLArantes/tucano-b3/commit/6268c2298f5aec7f2d95ff024b77a38823f5e3ff))
* **observability:** add unit and integration tests for metrics and logging (M7.7) ([2484283](https://github.com/JoseLArantes/tucano-b3/commit/2484283c2f322fc532058b51137627ad750354c7))
* **parsers:** add regression tests for ISO 20022 XML, ZIP depth limits, and NULLS NOT DISTINCT ([de33876](https://github.com/JoseLArantes/tucano-b3/commit/de33876d8c2527e96a8f825945e91703d6e89244))
* **parsers:** align PriceReport fixture with catalog bvmf.217.01 and label instrument fixture synthetic ([9b7fd6c](https://github.com/JoseLArantes/tucano-b3/commit/9b7fd6c3dc13a36f3ad41d9557d6f73fb8a02ef6))


### Build System

* **compose:** align docker-compose.yml with current in-process analytical architecture ([4844ae6](https://github.com/JoseLArantes/tucano-b3/commit/4844ae611aa426b3bf7ec8c88b5595e3d5bcd48b))
* **docker:** optimize Dockerfile for production and configure compose profiles (M8.4) ([27ebb04](https://github.com/JoseLArantes/tucano-b3/commit/27ebb04f4436ddd6b999ea82995ab2ad809ad7f3))


### Continuous Integration

* add automated multi-platform release workflow (M8.3) ([ab18217](https://github.com/JoseLArantes/tucano-b3/commit/ab18217b80a641c93c77df823c1c609db1edf5ac))
* add performance gate job for benchmarks, binary size, and startup time (M9.6) ([2702bee](https://github.com/JoseLArantes/tucano-b3/commit/2702beed320ea615b7a6a17a4538bd88746d02b9))
* **github:** automate docker build and push to ghcr on push to dev ([aca5412](https://github.com/JoseLArantes/tucano-b3/commit/aca54126ca35d92a9658054d1bf10d2e1aeb7ee3))
* update cross-platform CI workflow for Linux, macOS, and Windows (M8.2) ([b635da9](https://github.com/JoseLArantes/tucano-b3/commit/b635da99ea1bd704d7408857b594a89d5693c53f))

## [0.1.0] - 2026-11-11

### Added
- COTAHIST REST endpoints (tickers, quotes, latest, date queries)
- IBOV REST endpoints (history, latest)
- In-memory caching with TTL and LRU eviction
- Rate limiting with API key support
- OpenAPI 3.0 documentation with Swagger UI
- Prometheus metrics endpoint
- Structured JSON logging
- Docker deployment
- Cross-platform release builds (Linux, macOS, Windows)
- 100% test coverage on all business logic
