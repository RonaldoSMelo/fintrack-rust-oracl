WHENEVER SQLERROR EXIT SQL.SQLCODE ROLLBACK

-- Valores ficticios para testar o catalogo e o registro de compras.
MERGE INTO assets a
USING (
    SELECT 'Tesouro IPCA+' AS name, 125.00 AS unit_value FROM dual
    UNION ALL SELECT 'Tesouro Prefixado', 98.50 FROM dual
    UNION ALL SELECT 'CDB Demo', 100.00 FROM dual
    UNION ALL SELECT 'ETF Demo', 42.75 FROM dual
    UNION ALL SELECT 'FII Demo', 87.30 FROM dual
    UNION ALL SELECT 'Acao Demo', 31.20 FROM dual
    UNION ALL SELECT 'Bitcoin (BTC)', 350000.00 FROM dual
    UNION ALL SELECT 'Ethereum (ETH)', 12000.00 FROM dual
    UNION ALL SELECT 'Solana (SOL)', 750.00 FROM dual
    UNION ALL SELECT 'Cardano (ADA)', 2.50 FROM dual
) catalog
ON (a.name = catalog.name)
WHEN NOT MATCHED THEN
    INSERT (id, name, unit_value)
    VALUES (assets_seq.NEXTVAL, catalog.name, catalog.unit_value);

COMMIT;
