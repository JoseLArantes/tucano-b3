use tucano_b3::models::pagination::{PaginationMeta, PaginationParams};

#[test]
fn test_pagination_defaults() {
    let params = PaginationParams {
        pagina: 0,
        por_pagina: 0,
    };
    let validated = params.validated();
    assert_eq!(validated.pagina, 1);
    assert_eq!(validated.por_pagina, 1);
}

#[test]
fn test_pagination_clamp_max() {
    let params = PaginationParams {
        pagina: 1,
        por_pagina: 9999,
    };
    let validated = params.validated();
    assert_eq!(validated.por_pagina, PaginationParams::MAX_PER_PAGE);
}

#[test]
fn test_pagination_offset() {
    let params = PaginationParams {
        pagina: 3,
        por_pagina: 50,
    };
    assert_eq!(params.offset(), 100);
    assert_eq!(params.limit(), 50);
}

#[test]
fn test_pagination_meta() {
    let meta = PaginationMeta::new(252, 1, 50);
    assert_eq!(meta.total, 252);
    assert_eq!(meta.pagina, 1);
    assert_eq!(meta.por_pagina, 50);
    assert_eq!(meta.total_paginas, 6);
}

#[test]
fn test_pagination_meta_zero_per_page() {
    let meta = PaginationMeta::new(100, 1, 0);
    assert_eq!(meta.total_paginas, 0);
}
