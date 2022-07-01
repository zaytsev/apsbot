
CREATE TABLE menu_item(
    id INTEGER PRIMARY KEY,
    title TEXT,
    icon TEXT,
    price_min INTEGER,
    price_max INTEGER,
    duration_min INTEGER,
    duration_max INTEGER,
    parent_id INTEGER REFERENCES menu_item
);

CREATE INDEX menu_item_parent_id_idx ON menu_item (parent_id);
