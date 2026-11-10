//! Pagination model shared by all list endpoints.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Pagination query parameters.
///
/// Used by all list endpoints:
///   GET /api/v1/cotahist/tickers?pagina=1&por_pagina=50
///   GET /api/v1/cotahist/tickers/VALE3/quotes?pagina=1&por_pagina=252
#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub pagina: u32,

    /// Items per page
    #[serde(default = "default_per_page")]
    pub por_pagina: u32,
}

fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    50
}

impl PaginationParams {
    /// Maximum allowed items per page.
    pub const MAX_PER_PAGE: u32 = 1000;

    /// Validate and clamp pagination parameters.
    pub fn validated(&self) -> Self {
        Self {
            pagina: self.pagina.max(1),
            por_pagina: self.por_pagina.clamp(1, Self::MAX_PER_PAGE),
        }
    }

    /// SQL OFFSET value.
    pub fn offset(&self) -> u32 {
        (self.validated().pagina - 1) * self.validated().por_pagina
    }

    /// SQL LIMIT value.
    pub fn limit(&self) -> u32 {
        self.validated().por_pagina
    }
}

/// Pagination metadata included in list responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationMeta {
    pub total: usize,
    pub pagina: u32,
    pub por_pagina: u32,
    pub total_paginas: u32,
}

impl PaginationMeta {
    pub fn new(total: usize, pagina: u32, por_pagina: u32) -> Self {
        let total_paginas = if por_pagina > 0 {
            (total as f64 / por_pagina as f64).ceil() as u32
        } else {
            0
        };
        Self {
            total,
            pagina,
            por_pagina,
            total_paginas,
        }
    }
}
