ALTER TABLE employee_loads ADD COLUMN store_id uuid REFERENCES stores (id);

INSERT INTO stores (name)
SELECT DISTINCT store_name FROM employee_loads
ON CONFLICT (name) DO NOTHING;

UPDATE employee_loads
SET store_id = stores.id
FROM stores
WHERE employee_loads.store_name = stores.name;

ALTER TABLE employee_loads ALTER COLUMN store_id SET NOT NULL;
ALTER TABLE employee_loads DROP COLUMN store_name;
