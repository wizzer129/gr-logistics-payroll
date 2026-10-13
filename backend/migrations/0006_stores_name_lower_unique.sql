-- Store lookup is case-insensitive (resolve_store_id), so uniqueness must be too: with only
-- UNIQUE (name), two concurrent loads for "Walmart 12" and "walmart 12" could both miss the
-- lookup and insert duplicates. The expression index subsumes the old constraint.
CREATE UNIQUE INDEX stores_name_lower_key ON stores (lower(name));
ALTER TABLE stores DROP CONSTRAINT stores_name_key;
