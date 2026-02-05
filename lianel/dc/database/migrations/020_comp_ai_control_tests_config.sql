-- G2: Add config JSONB to control_tests for test parameters (e.g. GitHub owner/repo).
-- Enables runner to execute real integration tests (github_last_commit, github_branch_protection).

ALTER TABLE comp_ai.control_tests
ADD COLUMN IF NOT EXISTS config JSONB DEFAULT NULL;

COMMENT ON COLUMN comp_ai.control_tests.config IS 'Test-specific parameters, e.g. {"owner": "org", "repo": "repo"} for GitHub tests';

-- Optional: seed one GitHub test (replace owner/repo with your org) so runner can execute a real check.
INSERT INTO comp_ai.control_tests (control_id, name, test_type, schedule, config, last_result, last_details)
SELECT c.id, 'GitHub last commit (default branch)', 'github_last_commit', '0 6 * * *', '{"owner": "NimaLAN74", "repo": "hosting-base"}'::jsonb, NULL, NULL
FROM comp_ai.controls c LIMIT 1
ON CONFLICT (control_id, name) DO UPDATE SET config = EXCLUDED.config, test_type = EXCLUDED.test_type;
