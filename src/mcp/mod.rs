pub mod tools;

use serde_json::{json, Value};
use sqlx::PgPool;
use std::io::{self, BufRead, Write};
use tracing::{error, info};

pub struct McpServer {
    pool: PgPool,
}

impl McpServer {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run_stdio(&self) -> anyhow::Result<()> {
        info!("Starting Tucano B3 MCP Server (stdio mode)...");

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line_res in stdin.lock().lines() {
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let req: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    error!("Invalid JSON-RPC input: {}", e);
                    continue;
                }
            };

            let id = req.get("id").cloned().unwrap_or(Value::Null);
            let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

            match method {
                "initialize" => {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "protocolVersion": "2024-11-05",
                            "capabilities": {
                                "tools": {}
                            },
                            "serverInfo": {
                                "name": "tucano-b3-mcp",
                                "version": "0.1.0"
                            }
                        }
                    });
                    writeln!(stdout, "{}", resp)?;
                    stdout.flush()?;
                }
                "notifications/initialized" => {
                    // No response required for notifications
                }
                "tools/list" => {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "tools": [
                                {
                                    "name": "healthcheck",
                                    "description": "Verifica a conectividade do banco de dados e integridade do MCP server Tucano B3.",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {}
                                    }
                                },
                                {
                                    "name": "search_instruments",
                                    "description": "Busca instrumentos da B3 por ticker, ISIN, security_id ou nome da empresa/ativo.",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "query": { "type": "string", "description": "Termo de busca (ex: 'PETR4', 'VALE', 'BRPETR')" },
                                            "limit": { "type": "integer", "description": "Máximo de resultados (padrão 20)" }
                                        },
                                        "required": ["query"]
                                    }
                                },
                                {
                                    "name": "get_instrument_coverage",
                                    "description": "Retorna o período de cobertura e quantidade de cotações disponíveis para um ticker.",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "ticker": { "type": "string", "description": "Código do ticker (ex: 'PETR4')" }
                                        },
                                        "required": ["ticker"]
                                    }
                                },
                                {
                                    "name": "get_ohlcv_series",
                                    "description": "Retorna série diária OHLCV de cotações históricas nominais da B3 (COTAHIST).",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "ticker": { "type": "string", "description": "Código do ticker (ex: 'PETR4')" },
                                            "start_date": { "type": "string", "description": "Data inicial YYYY-MM-DD" },
                                            "end_date": { "type": "string", "description": "Data final YYYY-MM-DD" },
                                            "limit": { "type": "integer", "description": "Limite de registros (padrão 250)" }
                                        },
                                        "required": ["ticker"]
                                    }
                                },
                                {
                                    "name": "get_index_portfolio",
                                    "description": "Consulta a composição da carteira teórica de um índice da B3 (ex: IBOV, SMLL).",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "index_code": { "type": "string", "description": "Código do índice (ex: 'IBOV')" }
                                        },
                                        "required": ["index_code"]
                                    }
                                },
                                {
                                    "name": "get_dataset_diagnostics",
                                    "description": "Retorna métricas de volume e datas mínimas/máximas de todos os conjuntos de dados da B3.",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {}
                                    }
                                }
                            ]
                        }
                    });
                    writeln!(stdout, "{}", resp)?;
                    stdout.flush()?;
                }
                "tools/call" => {
                    let params = req.get("params").cloned().unwrap_or(Value::Null);
                    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

                    let result_data = match tool_name {
                        "healthcheck" => tools::handle_healthcheck(&self.pool).await,
                        "search_instruments" => tools::handle_search_instruments(&self.pool, &arguments).await,
                        "get_instrument_coverage" => tools::handle_get_instrument_coverage(&self.pool, &arguments).await,
                        "get_ohlcv_series" => tools::handle_get_ohlcv_series(&self.pool, &arguments).await,
                        "get_index_portfolio" => tools::handle_get_index_portfolio(&self.pool, &arguments).await,
                        "get_dataset_diagnostics" => tools::handle_get_dataset_diagnostics(&self.pool).await,
                        _ => json!({ "error": format!("Ferramenta desconhecida: {tool_name}") }),
                    };

                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&result_data)?
                                }
                            ]
                        }
                    });
                    writeln!(stdout, "{}", resp)?;
                    stdout.flush()?;
                }
                _ => {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method not found: {method}")
                        }
                    });
                    writeln!(stdout, "{}", resp)?;
                    stdout.flush()?;
                }
            }
        }

        Ok(())
    }
}

