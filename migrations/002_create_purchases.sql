WHENEVER SQLERROR EXIT SQL.SQLCODE

CREATE TABLE IF NOT EXISTS purchases (
    id NUMBER(19) GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id NUMBER(19) NOT NULL REFERENCES users(id),
    asset_id NUMBER(19) NOT NULL REFERENCES assets(id),
    quantity NUMBER(18, 8) NOT NULL CHECK (quantity > 0),
    unit_price NUMBER(18, 8) NOT NULL CHECK (unit_price > 0),
    purchased_at TIMESTAMP DEFAULT SYS_EXTRACT_UTC(SYSTIMESTAMP) NOT NULL
);

CREATE INDEX IF NOT EXISTS purchases_user_asset_idx ON purchases (user_id, asset_id);
CREATE INDEX IF NOT EXISTS purchases_asset_idx ON purchases (asset_id);
