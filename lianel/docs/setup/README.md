# Setup Documentation

This directory contains setup and configuration documentation.

## Files

- **ADMIN-CREDENTIALS.md** - Admin login credentials and access information
- **SSO-RESET-SUMMARY.md** - Summary of SSO reset process
- **KEYCLOAK-RESET-COMPLETE.md** - Keycloak reset completion guide
- **.env.example** - Example environment file with all secrets (DO NOT COMMIT ACTUAL .env)

## Environment Variables

The `.env.example` file contains all required environment variables for:
- Keycloak configuration
- Database credentials
- OAuth client secrets
- Service configurations

**⚠️ Important**: Never commit the actual `.env` file to git. Use `.env.example` as a template.

