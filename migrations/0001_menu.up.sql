
CREATE TABLE menu_item(
    id BIGINT PRIMARY KEY,
    title TEXT NOT NULL,
    icon TEXT,
    price_min INT,
    price_max INT,
    duration_min BIGINT,
    duration_max BIGINT,
    parent_id BIGINT REFERENCES menu_item,
    org_id BIGINT NOT NULL
);

CREATE INDEX menu_item_parent_id_idx ON menu_item (parent_id);
CREATE INDEX menu_item_org_id_idx ON menu_item (org_id);
