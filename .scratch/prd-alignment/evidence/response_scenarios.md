# Tucano B3 - Live Response Traces and Scenarios (R1)

Captured against live service `b3_api` at commit `1e7ec9de97466376e85f37cdfad6a0615ed80641` (port 8000/8010).

---

## Scenario 1: Unauthenticated Protected Endpoint (401 Unauthorized)

### Request:
```http
GET /api/v1/b3/instruments HTTP/1.1
Host: localhost:8010
```

### Response:
```http
HTTP/1.1 401 Unauthorized
content-type: application/json

{
  "error": "Unauthorized",
  "message": "Token de autenticação ausente ou inválido. Use 'Authorization: Bearer <token>'."
}
```

---

## Scenario 2: Authenticated Paginated Query with Provenance (200 OK)

### Request:
```http
GET /api/v1/b3/instruments?limit=2 HTTP/1.1
Host: localhost:8010
Authorization: Bearer dev-token
```

### Response:
```http
HTTP/1.1 200 OK
content-type: application/json

{
  "items": [
    {
      "id": "e18c6ce0-05bf-43ee-8dd6-72b3e31fdd44",
      "security_id": "10000001",
      "security_source": null,
      "market_identifier_code": null,
      "asset": "CASH",
      "asset_description": "CASH",
      "market_code": "1",
      "market_name": null,
      "segment_code": "5",
      "segment_name": null,
      "instrument_type": null,
      "security_category": "37",
      "cfi_code": "MMXXXX",
      "instrument_status": null,
      "maturity_date": null,
      "ticker_symbol": null,
      "isin": null,
      "provenance": {
        "source_file_type": "BVBG_028_INSTRUMENTS_FILE",
        "source_file_id": "b7003950-1722-4421-938d-6678f5b513b6",
        "source_file_name": "BVBG_028_INSTRUMENTS_FILE_2026-11-05",
        "source_member_name": "BVBG.028.02_BV000327202611050327106857929518813.xml",
        "reference_date": "2026-11-05",
        "sha256": "9011dd3acccb14c17c20d79a486b92359ceb63f7c7d19a6d1f87a9bbefb6f013",
        "ingested_at": "2026-11-05T23:42:43.524089Z"
      }
    }
  ],
  "total": 123820,
  "limit": 2,
  "offset": 0
}
```

---

## Scenario 3: COTAHIST Quotes with Mandatory Regulatory Disclaimer (200 OK)

### Request:
```http
GET /api/v1/b3/cotahist/quotes?limit=1 HTTP/1.1
Host: localhost:8010
Authorization: Bearer dev-token
```

### Response:
```http
HTTP/1.1 200 OK
content-type: application/json

{
  "items": [
    {
      "id": "8561a3d7-1484-4f6f-b392-ca445d98e8e1",
      "instrument_id": "c4351b46-7588-410c-b614-144dc8a277e2",
      "trade_date": "2026-02-02",
      "bdi_code": "02",
      "ticker_symbol": "VALE3",
      "market_type": "010",
      "issuer_short_name": "VALE",
      "paper_specification": "ON",
      "term_days": "000",
      "currency": "BRL",
      "open_price": "60.0000000000",
      "high_price": "61.5000000000",
      "low_price": "59.5000000000",
      "average_price": "60.5000000000",
      "last_price": "61.0000000000",
      "best_bid_price": "60.9000000000",
      "best_ask_price": "61.1000000000",
      "trades_count": 123,
      "traded_quantity": "10000000.000000",
      "traded_volume": "610000000.000000",
      "option_exercise_price": "0",
      "price_correction_indicator": "0",
      "maturity_date": null,
      "quotation_factor": 1,
      "exercise_price_points": "0",
      "isin": "BRVALEACNOR0",
      "distribution_number": "123",
      "resolution_status": "resolved",
      "provenance": {
        "source_file_type": "COTAHIST_DAILY",
        "source_file_id": "4ae3486d-1930-4fa5-8703-e9fad611e454",
        "source_file_name": "COTAHIST_DAILY_2026-02-02",
        "source_member_name": "COTAHIST_DAILY",
        "reference_date": "2026-02-02",
        "sha256": "6192357443d85da9780322b24baf34b77ca34f330336613a3f35bbb50fb9cf5c",
        "ingested_at": "2026-11-06T12:43:55.531016Z"
      }
    }
  ],
  "total": 1,
  "limit": 1,
  "offset": 0,
  "disclaimer": "AVISO: Preços brutos nominais de negociação histórica da B3. Não ajustados retroativamente por proventos em dinheiro (dividendos/JCP), desdobramentos (splits), grupamentos ou bonificações corporativas."
}
```

---

## Scenario 4: Non-existent Resource (404 Not Found)

### Request:
```http
GET /api/v1/b3/instruments/invalid-uuid-format HTTP/1.1
Host: localhost:8010
Authorization: Bearer dev-token
```

### Response:
```http
HTTP/1.1 404 Not Found
content-type: text/plain; charset=utf-8

Instrument not found
```

