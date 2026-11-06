pub mod tools;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

pub struct McpServer {
    pool: PgPool,
}
impl McpServer {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn run_stdio(&self) -> anyhow::Result<()> {
        let mut input = BufReader::new(tokio::io::stdin());
        let mut output = tokio::io::stdout();
        let mut initialized = false;
        loop {
            let mut line = Vec::new();
            let read = (&mut input)
                .take(1_048_577)
                .read_until(b'\n', &mut line)
                .await?;
            if read == 0 {
                break;
            }
            if line.len() > 1_048_576 {
                anyhow::bail!("Mensagem MCP excede o limite de bytes.");
            }
            let request: Value = match serde_json::from_slice(&line) {
                Ok(value) => value,
                Err(_) => {
                    write_response(&mut output,json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"JSON inválido."}})).await?;
                    continue;
                }
            };
            let id = request.get("id").cloned();
            let method = request.get("method").and_then(Value::as_str).unwrap_or("");
            if id.is_none() {
                if method == "notifications/initialized" {
                    initialized = true;
                }
                continue;
            }
            let id = id.unwrap_or(Value::Null);
            let result = match method {
                "initialize" => {
                    let version = request
                        .pointer("/params/protocolVersion")
                        .and_then(Value::as_str)
                        .unwrap_or("2024-11-05");
                    let supported = if matches!(version, "2024-11-05" | "2025-03-26" | "2025-06-18")
                    {
                        version
                    } else {
                        "2024-11-05"
                    };
                    Ok(
                        json!({"protocolVersion":supported,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"tucano-b3","version":env!("CARGO_PKG_VERSION")}}),
                    )
                }
                "ping" => Ok(json!({})),
                "tools/list" if initialized => Ok(json!({"tools":tools::tool_list()})),
                "tools/call" if initialized => {
                    let name = request
                        .pointer("/params/name")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let arguments = request
                        .pointer("/params/arguments")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    match tools::call(&self.pool, name, arguments).await {
                        Ok(value) => Ok(
                            json!({"content":[{"type":"text","text":serde_json::to_string(&value)?}],"isError":false}),
                        ),
                        Err(error) => Ok(
                            json!({"content":[{"type":"text","text":error.to_string()}],"isError":true}),
                        ),
                    }
                }
                "tools/list" | "tools/call" => Err((-32002, "Cliente não inicializado.")),
                _ => Err((-32601, "Método inexistente ou não permitido.")),
            };
            let response = match result {
                Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
                Err((code, message)) => {
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
                }
            };
            write_response(&mut output, response).await?;
        }
        Ok(())
    }
}
async fn write_response(output: &mut tokio::io::Stdout, value: Value) -> anyhow::Result<()> {
    output
        .write_all(serde_json::to_string(&value)?.as_bytes())
        .await?;
    output.write_all(b"\n").await?;
    output.flush().await?;
    Ok(())
}
