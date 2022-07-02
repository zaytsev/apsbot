
CREATE TABLE menu_item(
    id BIGINT PRIMARY KEY,
    title TEXT,
    icon TEXT,
    price_min BIGINT,
    price_max BIGINT,
    duration_min BIGINT,
    duration_max BIGINT,
    parent_id BIGINT REFERENCES menu_item
);

CREATE INDEX menu_item_parent_id_idx ON menu_item (parent_id);
