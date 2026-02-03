-- Grant comp_ai schema and table access to app user (POSTGRES_USER, default airflow).
-- Comp-AI service connects as POSTGRES_USER; without this, list_controls returns 500.
-- Conditional so CI (runner Postgres without airflow role) does not fail.

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'airflow') THEN
    EXECUTE 'GRANT USAGE ON SCHEMA comp_ai TO airflow';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA comp_ai TO airflow';
    EXECUTE 'GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA comp_ai TO airflow';
  END IF;
END
$$;
