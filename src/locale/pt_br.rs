//! Brazilian Portuguese locale strings for user-facing messages.
//!
//! All strings in this module are displayed to end users.
//! They MUST be in Brazilian Portuguese.

// === Error messages ===
pub const ERR_TICKER_NOT_FOUND: &str = "Ticker não encontrado: {ticker}";
pub const ERR_INVALID_DATE: &str = "Formato de data inválido: {value}";
pub const ERR_DATE_RANGE_INVALID: &str = "Intervalo de datas inválido";
pub const ERR_NO_DATA_IN_RANGE: &str = "Nenhum dado encontrado no período";
pub const ERR_RATE_LIMITED: &str = "Limite de requisições excedido";
pub const ERR_INTERNAL: &str = "Erro interno do servidor";
pub const ERR_SERVICE_UNAVAILABLE: &str = "Serviço indisponível";
pub const ERR_INVALID_PARAM: &str = "Parâmetro inválido: {message}";

// === Error details (hints) ===
pub const HINT_TICKER_NOT_FOUND: &str = "Verifique se o ticker existe nos dados ingeridos.";
pub const HINT_INVALID_DATE: &str = "Use o formato AAAA-MM-DD (ex: 2026-11-10).";
pub const HINT_DATE_RANGE_INVALID: &str = "A data inicial não pode ser posterior à data final.";
pub const HINT_NO_DATA_IN_RANGE: &str = "Verifique o intervalo de datas disponível nos dados.";
pub const HINT_RATE_LIMITED: &str = "Aguarde e tente novamente.";

// === Health endpoint ===
pub const HEALTH_STATUS_HEALTHY: &str = "healthy";
pub const HEALTH_STATUS_DEGRADED: &str = "degraded";
pub const HEALTH_STATUS_UNHEALTHY: &str = "unhealthy";
