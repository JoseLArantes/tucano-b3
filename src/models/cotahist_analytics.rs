//! Derived COTAHIST analytics response models.
//!
//! All indicators are computed from raw COTAHIST OHLCV fields and are **not**
//! official B3 published series. Every response includes `aviso` stating that
//! prices are nominal (unadjusted for corporate actions).

use serde::Serialize;
use utoipa::ToSchema;

use crate::models::pagination::PaginationMeta;

/// Standard COTAHIST nominal-price warning (Brazilian Portuguese).
pub const COTAHIST_AVISO_NOMINAL: &str = "Preços nominais históricos sem ajuste retroativo por eventos corporativos. Indicadores derivados não são séries oficiais publicadas pela B3.";

/// Period return summary for a ticker.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReturnsResponse {
    /// Código do ticker (ex: VALE3)
    #[schema(example = "VALE3")]
    pub ticker: String,
    /// Data inicial solicitada (AAAA-MM-DD)
    #[schema(example = "2026-01-01")]
    pub desde: String,
    /// Data final solicitada, se informada (AAAA-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "2026-01-31")]
    pub ate: Option<String>,
    /// Primeiro pregão com fechamento válido no período (AAAA-MM-DD)
    #[schema(example = "2026-01-02")]
    pub data_inicial: String,
    /// Último pregão com fechamento válido no período (AAAA-MM-DD)
    #[schema(example = "2026-01-31")]
    pub data_final: String,
    /// Preço de fechamento (PREULT) no primeiro pregão válido
    #[schema(example = 61.5)]
    pub pre_ult_inicial: f64,
    /// Preço de fechamento (PREULT) no último pregão válido
    #[schema(example = 62.3)]
    pub pre_ult_final: f64,
    /// Quantidade de pregões com `pre_ult` no período
    #[schema(example = 20)]
    pub pregoes: u32,
    /// Variação absoluta de fechamento: `pre_ult_final - pre_ult_inicial` (R$)
    #[schema(example = 0.8)]
    pub retorno_abs: f64,
    /// Retorno percentual simples: `((final/inicial) - 1) * 100`
    #[schema(example = 1.3008)]
    pub retorno_pct: f64,
    /// Retorno anualizado percentual assumindo 252 pregoes/ano: `((1+r)^(252/n)-1)*100`, onde `n` é `pregoes - 1`. Ausente se `pregoes < 2`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 17.5)]
    pub retorno_anualizado_pct: Option<f64>,
    /// Aviso sobre preços nominais e natureza derivada do indicador
    pub aviso: String,
}

/// Aggregate market-card statistics for a ticker in a period.
#[derive(Debug, Serialize, ToSchema)]
pub struct StatsResponse {
    /// Código do ticker
    #[schema(example = "VALE3")]
    pub ticker: String,
    /// Data inicial solicitada (AAAA-MM-DD)
    pub desde: String,
    /// Data final solicitada, se informada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Quantidade de pregoes no período
    pub pregoes: u32,
    /// Primeiro pregão (AAAA-MM-DD)
    pub data_inicial: String,
    /// Último pregão (AAAA-MM-DD)
    pub data_final: String,
    /// Fechamento no primeiro pregão
    pub pre_ult_inicial: f64,
    /// Fechamento no último pregão
    pub pre_ult_final: f64,
    /// Mínimo de PREULT no período
    pub pre_ult_min: f64,
    /// Data do mínimo de PREULT (empate: data mais antiga)
    pub data_pre_ult_min: String,
    /// Máximo de PREULT no período
    pub pre_ult_max: f64,
    /// Data do máximo de PREULT (empate: data mais antiga)
    pub data_pre_ult_max: String,
    /// Média aritmética de PREULT
    pub pre_ult_medio: f64,
    /// Mínimo de PREMIN (mínima intradiária) no período, se disponível
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_min_periodo: Option<f64>,
    /// Máximo de PREMAX (máxima intradiária) no período, se disponível
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_max_periodo: Option<f64>,
    /// Retorno percentual simples no período
    pub retorno_pct: f64,
    /// Volatilidade realizada anualizada (%) a partir dos retornos diários de PREULT (desvio-padrão amostral × √252). Ausente com menos de 2 retornos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volatilidade_anualizada_pct: Option<f64>,
    /// Média de True Range aproximado (PREMAX−PREMIN; fallback |PREULT−PREULT anterior|) no período
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atr_medio: Option<f64>,
    /// Volume financeiro médio diário (VOLTOT) em R$
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_tot_medio: Option<f64>,
    /// Quantidade média de títulos negociados (QUATTOT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quat_tot_medio: Option<f64>,
    /// Número médio de negócios (TOTNEG)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tot_neg_medio: Option<f64>,
    /// Mediana de VOLTOT no período
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_tot_mediano: Option<f64>,
    /// Fração de pregoes com `tot_neg` nulo ou zero (0–1)
    pub fracao_pregoes_sem_negocio: f64,
    /// Aviso sobre preços nominais
    pub aviso: String,
}

/// Maximum drawdown analysis on nominal closes.
#[derive(Debug, Serialize, ToSchema)]
pub struct DrawdownResponse {
    /// Código do ticker
    pub ticker: String,
    /// Data inicial solicitada
    pub desde: String,
    /// Data final solicitada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Máximo drawdown percentual (valor negativo ou zero): `(vale − pico) / pico * 100`
    #[schema(example = -12.5)]
    pub max_drawdown_pct: f64,
    /// Queda absoluta no máximo drawdown: `vale − pico` (R$)
    pub max_drawdown_abs: f64,
    /// Data do pico de fechamento que antecede o máximo drawdown
    pub data_pico: String,
    /// PREULT no pico
    pub pre_ult_pico: f64,
    /// Data do vale (mínimo relativo após o pico)
    pub data_vale: String,
    /// PREULT no vale
    pub pre_ult_vale: f64,
    /// Pregões entre pico e vale (inclusive)
    pub pregoes_submersos: u32,
    /// Drawdown percentual atual em relação ao máximo PREULT do período até o último pregão
    pub drawdown_atual_pct: f64,
    /// PREULT do último pregão
    pub pre_ult_atual: f64,
    /// Máximo PREULT observado até o último pregão do período
    pub pre_ult_max_ate_atual: f64,
    /// Aviso sobre preços nominais
    pub aviso: String,
}

/// Position of the latest close within the period high–low range.
#[derive(Debug, Serialize, ToSchema)]
pub struct RangePositionResponse {
    /// Código do ticker
    pub ticker: String,
    /// Data inicial solicitada
    pub desde: String,
    /// Data final solicitada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Data do último pregão considerado
    pub data_referencia: String,
    /// PREULT de referência (último pregão)
    pub pre_ult: f64,
    /// Mínimo de PREULT no período
    pub pre_ult_min: f64,
    /// Data do mínimo
    pub data_pre_ult_min: String,
    /// Máximo de PREULT no período
    pub pre_ult_max: f64,
    /// Data do máximo
    pub data_pre_ult_max: String,
    /// Posição no intervalo \[mínimo, máximo\] em percentual: `((pre_ult − min) / (max − min)) * 100`. `null` se max = min.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub posicao_pct: Option<f64>,
    /// Distância percentual ao máximo: `((pre_ult / max) − 1) * 100`
    pub distancia_max_pct: f64,
    /// Distância percentual ao mínimo: `((pre_ult / min) − 1) * 100`
    pub distancia_min_pct: f64,
    /// Aviso sobre preços nominais
    pub aviso: String,
}

/// Relative performance versus a benchmark (IBOV from COTAHIST-era IBOV parquet).
#[derive(Debug, Serialize, ToSchema)]
pub struct RelativeResponse {
    /// Código do ticker
    pub ticker: String,
    /// Benchmark utilizado (`IBOV`)
    #[schema(example = "IBOV")]
    pub benchmark: String,
    /// Data inicial solicitada
    pub desde: String,
    /// Data final solicitada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Primeira data comum com fechamento válido para ticker e benchmark
    pub data_inicial: String,
    /// Última data comum com fechamento válido
    pub data_final: String,
    /// Pregões alinhados (interseção de datas)
    pub pregoes_alinhados: u32,
    /// Retorno percentual do ticker no intervalo alinhado
    pub retorno_ticker_pct: f64,
    /// Retorno percentual do benchmark no intervalo alinhado
    pub retorno_benchmark_pct: f64,
    /// Excesso de retorno: `retorno_ticker_pct − retorno_benchmark_pct`
    pub excesso_retorno_pct: f64,
    /// PREULT do ticker na primeira data alinhada
    pub pre_ult_ticker_inicial: f64,
    /// PREULT do ticker na última data alinhada
    pub pre_ult_ticker_final: f64,
    /// PREULT do benchmark na primeira data alinhada
    pub pre_ult_benchmark_inicial: f64,
    /// PREULT do benchmark na última data alinhada
    pub pre_ult_benchmark_final: f64,
    /// Aviso sobre preços nominais
    pub aviso: String,
}

/// Single session mover row.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MoverItem {
    /// Código do ticker
    pub ticker: String,
    /// Data do pregão
    pub data: String,
    /// Fechamento (PREULT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_ult: Option<f64>,
    /// Variação percentual vs. pregão anterior do mesmo ticker
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var_pct: Option<f64>,
    /// Variação absoluta vs. pregão anterior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var_abs: Option<f64>,
    /// Volume financeiro (VOLTOT) em R$
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_tot: Option<f64>,
    /// Quantidade de negócios (TOTNEG)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tot_neg: Option<u32>,
    /// Quantidade de títulos (QUATTOT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quat_tot: Option<u64>,
    /// Tipo de mercado B3 (TPMERC), quando disponível
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_type: Option<u8>,
}

/// Session movers (gainers, losers, volume, trades).
#[derive(Debug, Serialize, ToSchema)]
pub struct MoversResponse {
    /// Data do pregão (AAAA-MM-DD)
    pub data: String,
    /// Ranking solicitado: `gainers`, `losers`, `volume` ou `trades`
    pub ranking: String,
    /// Itens ordenados conforme o ranking
    pub itens: Vec<MoverItem>,
    /// Aviso sobre preços nominais / variações derivadas
    pub aviso: String,
}

/// Liquidity screen row for one ticker.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LiquidityItem {
    /// Código do ticker
    pub ticker: String,
    /// Pregões considerados no período
    pub pregoes: u32,
    /// Volume financeiro médio diário (ADV em R$)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_tot_medio: Option<f64>,
    /// Proxy de mediana de VOLTOT (hoje igual à média; reservado para mediana exata futura)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_tot_mediano: Option<f64>,
    /// Quantidade média de títulos
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quat_tot_medio: Option<f64>,
    /// Média de negócios
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tot_neg_medio: Option<f64>,
    /// Proxy de mediana de TOTNEG (hoje igual à média; reservado para mediana exata futura)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tot_neg_mediano: Option<f64>,
    /// Fração de pregoes sem negócio (0–1)
    pub fracao_pregoes_sem_negocio: f64,
}

/// Cross-sectional liquidity screen.
#[derive(Debug, Serialize, ToSchema)]
pub struct LiquidityScreenResponse {
    /// Data inicial solicitada
    pub desde: String,
    /// Data final solicitada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Filtro opcional de `market_type` (TPMERC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_type: Option<u8>,
    /// Resultados paginados
    pub itens: Vec<LiquidityItem>,
    /// Metadados de paginação
    pub paginacao: PaginationMeta,
    /// Aviso sobre natureza operacional dos indicadores
    pub aviso: String,
}

/// Opening gap versus previous close.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GapItem {
    /// Data do pregão em que o gap é observado (abertura)
    pub data: String,
    /// Abertura (PREABR)
    pub pre_abr: f64,
    /// Fechamento do pregão anterior (PREULT)
    pub pre_ult_anterior: f64,
    /// Gap absoluto: `pre_abr − pre_ult_anterior`
    pub gap_abs: f64,
    /// Gap percentual: `(pre_abr / pre_ult_anterior − 1) * 100`
    pub gap_pct: f64,
    /// Direção: `alta`, `baixa` ou `neutro`
    pub direcao: String,
    /// Se o pregão preencheu o gap (mínima ≤ pre_ult_anterior ≤ máxima, conforme direção)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preenchido: Option<bool>,
}

/// Gap analysis response.
#[derive(Debug, Serialize, ToSchema)]
pub struct GapsResponse {
    /// Código do ticker
    pub ticker: String,
    /// Data inicial solicitada
    pub desde: String,
    /// Data final solicitada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Filtro de magnitude mínima absoluta de `gap_pct`, se aplicado
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_pct_min: Option<f64>,
    /// Gaps detectados (ordenados por data ascendente)
    pub gaps: Vec<GapItem>,
    /// Aviso sobre preços nominais
    pub aviso: String,
}

/// One point of rolling technical indicators.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RollingPoint {
    /// Data do pregão (AAAA-MM-DD)
    pub data: String,
    /// Fechamento (PREULT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_ult: Option<f64>,
    /// Média móvel simples de PREULT na janela
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sma: Option<f64>,
    /// Média móvel exponencial de PREULT (α = 2/(janela+1))
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ema: Option<f64>,
    /// Volatilidade realizada anualizada (%) na janela de retornos diários
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volatilidade_anualizada_pct: Option<f64>,
    /// Média móvel de VOLTOT (ADV parcial) na janela
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adv_vol_tot: Option<f64>,
}

/// Rolling indicators series.
#[derive(Debug, Serialize, ToSchema)]
pub struct RollingResponse {
    /// Código do ticker
    pub ticker: String,
    /// Data inicial solicitada
    pub desde: String,
    /// Data final solicitada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Janela em pregoes
    pub janela: u32,
    /// Pontos diários (ascendente). Valores móveis ficam `null` até a janela estar completa (exceto EMA após seed).
    pub series: Vec<RollingPoint>,
    /// Aviso: indicadores derivados, não oficiais
    pub aviso: String,
}

/// Bid/ask spread point.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SpreadPoint {
    /// Data do pregão
    pub data: String,
    /// Melhor oferta de compra (PREOFECOMPRA)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_ofe_compra: Option<f64>,
    /// Melhor oferta de venda (PREOFEVENDA)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_ofe_venda: Option<f64>,
    /// Spread absoluto: venda − compra (quando ambos existem)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread_abs: Option<f64>,
    /// Spread percentual sobre o mid: `((venda−compra)/((venda+compra)/2))*100`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread_pct: Option<f64>,
    /// Preço médio entre compra e venda
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mid: Option<f64>,
}

/// Bid/ask spread series.
#[derive(Debug, Serialize, ToSchema)]
pub struct SpreadResponse {
    /// Código do ticker
    pub ticker: String,
    /// Data inicial solicitada
    pub desde: String,
    /// Data final solicitada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Pontos diários
    pub series: Vec<SpreadPoint>,
    /// Quantidade de pregoes com ambas as pontas disponíveis
    pub pregoes_com_spread: u32,
    /// Spread percentual médio (apenas pregoes com spread válido)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread_pct_medio: Option<f64>,
    /// Aviso sobre preços nominais / book de fechamento COTAHIST
    pub aviso: String,
}

/// Normalized compare series for one ticker (base 100 on first common date).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CompareSeries {
    /// Código do ticker
    pub ticker: String,
    /// PREULT na data base (primeiro pregão comum a todos)
    pub pre_ult_base: f64,
    /// PREULT no último pregão comum
    pub pre_ult_final: f64,
    /// Retorno percentual no intervalo comum
    pub retorno_pct: f64,
    /// Série normalizada (base 100) alinhada às `datas`
    pub valores_base_100: Vec<Option<f64>>,
}

/// Multi-ticker normalized comparison.
#[derive(Debug, Serialize, ToSchema)]
pub struct CompareResponse {
    /// Data inicial solicitada
    pub desde: String,
    /// Data final solicitada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Datas comuns (interseção) em ordem ascendente
    pub datas: Vec<String>,
    /// Uma série por ticker solicitado (apenas tickers com pelo menos um ponto)
    pub series: Vec<CompareSeries>,
    /// Aviso sobre preços nominais e normalização derivada
    pub aviso: String,
}
