CREATE TABLE users (
	id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	email text UNIQUE NOT NULL,
	password_hash text NOT NULL,
	role text NOT NULL DEFAULT 'admin',
	created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
	session_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
	expires_at timestamptz NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE employees (
	id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	first_name text NOT NULL,
	last_name text NOT NULL,
	dob date,
	email text,
	address text,
	active boolean NOT NULL DEFAULT true,
	created_by uuid REFERENCES users (id),
	last_edited_by uuid REFERENCES users (id),
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE employee_weekly_logs (
	id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	employee_id uuid NOT NULL REFERENCES employees (id),
	week_start date NOT NULL,
	week_end date NOT NULL,
	status text NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'completed')),
	summary_total numeric(10, 2),
	employee_cut numeric(10, 2),
	created_by uuid REFERENCES users (id),
	last_edited_by uuid REFERENCES users (id),
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now(),
	UNIQUE (employee_id, week_start)
);

CREATE TABLE employee_loads (
	id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	employee_id uuid NOT NULL REFERENCES employees (id),
	weekly_log_id uuid NOT NULL REFERENCES employee_weekly_logs (id) ON DELETE CASCADE,
	store_name text NOT NULL,
	load_date date NOT NULL,
	pay numeric(10, 2) NOT NULL CHECK (pay >= 0),
	extra_pay numeric(10, 2) NOT NULL DEFAULT 0 CHECK (extra_pay >= 0),
	created_by uuid REFERENCES users (id),
	last_edited_by uuid REFERENCES users (id),
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE audit_log (
	id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	entity text NOT NULL,
	entity_id uuid NOT NULL,
	action text NOT NULL CHECK (action IN ('create', 'update', 'delete')),
	changed_by uuid REFERENCES users (id),
	diff jsonb,
	created_at timestamptz NOT NULL DEFAULT now()
);
