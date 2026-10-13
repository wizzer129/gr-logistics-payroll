CREATE TABLE stores (
	id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	name text UNIQUE NOT NULL,
	created_by uuid REFERENCES users (id),
	last_edited_by uuid REFERENCES users (id),
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);
