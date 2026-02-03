-- Grant comp_ai schema and table access to app user (POSTGRES_USER, default airflow).
-- Comp-AI service connects as POSTGRES_USER; without this, list_controls returns 500.
-- If your app uses a different role, add GRANTs for it or run equivalent GRANTs manually.

GRANT USAGE ON SCHEMA comp_ai TO airflow;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA comp_ai TO airflow;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA comp_ai TO airflow;
