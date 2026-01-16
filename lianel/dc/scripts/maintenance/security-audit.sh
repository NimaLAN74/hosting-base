#!/bin/bash
# Security Audit Script
# Checks for common security issues in the deployment

set -e

echo "=========================================="
echo "Security Audit - Lianel Platform"
echo "Date: $(date)"
echo "=========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ISSUES=0
WARNINGS=0

# Function to check for issues
check_issue() {
    if [ $? -ne 0 ]; then
        echo -e "${RED}[FAIL]${NC} $1"
        ISSUES=$((ISSUES + 1))
        return 1
    else
        echo -e "${GREEN}[PASS]${NC} $1"
        return 0
    fi
}

check_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

echo "1. Checking for exposed secrets..."
echo "-----------------------------------"

# Check .env file for hardcoded secrets
if [ -f .env ]; then
    if grep -q "password.*=.*password" .env 2>/dev/null; then
        check_warning ".env contains weak passwords"
    fi
    
    if grep -q "secret.*=.*secret" .env 2>/dev/null; then
        check_warning ".env contains weak secrets"
    fi
    
    # Check if .env is in .gitignore
    if ! grep -q "^\.env$" .gitignore 2>/dev/null; then
        check_issue ".env file not in .gitignore"
    else
        check_issue ".env file is in .gitignore"
    fi
else
    check_warning ".env file not found"
fi

# Check for secrets in code
echo ""
echo "2. Checking for secrets in code..."
echo "-----------------------------------"

if grep -r "password.*=.*['\"].*password" --include="*.py" --include="*.rs" --include="*.js" --include="*.ts" . 2>/dev/null | grep -v ".git" | grep -v "node_modules" | head -5; then
    check_warning "Potential hardcoded passwords in code"
else
    check_issue "No obvious hardcoded passwords found"
fi

echo ""
echo "3. Checking SSL/TLS configuration..."
echo "-----------------------------------"

# Check nginx SSL configuration
if [ -f nginx/config/nginx.conf ]; then
    if grep -q "ssl_protocols.*TLSv1.0\|TLSv1.1" nginx/config/nginx.conf; then
        check_issue "Nginx uses insecure SSL protocols (TLSv1.0 or TLSv1.1)"
    else
        check_issue "Nginx SSL protocols are secure"
    fi
    
    if grep -q "server_tokens off" nginx/config/nginx.conf; then
        check_issue "Nginx version hiding is enabled"
    else
        check_warning "Nginx version hiding is not enabled"
    fi
else
    check_warning "nginx.conf not found"
fi

echo ""
echo "4. Checking security headers..."
echo "-----------------------------------"

if [ -f nginx/config/nginx.conf ]; then
    if grep -q "X-Frame-Options" nginx/config/nginx.conf; then
        check_issue "X-Frame-Options header is set"
    else
        check_warning "X-Frame-Options header is missing"
    fi
    
    if grep -q "X-Content-Type-Options" nginx/config/nginx.conf; then
        check_issue "X-Content-Type-Options header is set"
    else
        check_warning "X-Content-Type-Options header is missing"
    fi
    
    if grep -q "Strict-Transport-Security" nginx/config/nginx.conf; then
        check_issue "HSTS header is set"
    else
        check_warning "HSTS header is missing"
    fi
fi

echo ""
echo "5. Checking rate limiting..."
echo "-----------------------------------"

if [ -f nginx/config/nginx.conf ]; then
    if grep -q "limit_req_zone" nginx/config/nginx.conf; then
        check_issue "Rate limiting is configured"
    else
        check_warning "Rate limiting is not configured"
    fi
fi

echo ""
echo "6. Checking container security..."
echo "-----------------------------------"

# Check docker-compose files for security settings
for file in docker-compose*.yaml; do
    if [ -f "$file" ]; then
        if grep -q "read_only: true" "$file"; then
            check_issue "$file: Some containers use read-only filesystem"
        else
            check_warning "$file: No read-only filesystem configured"
        fi
        
        if grep -q "user:" "$file"; then
            check_issue "$file: Containers run as non-root user"
        else
            check_warning "$file: Containers may run as root"
        fi
    fi
done

echo ""
echo "7. Checking authentication configuration..."
echo "-----------------------------------"

# Check Keycloak configuration
if [ -f docker-compose.infra.yaml ]; then
    if grep -q "KC_HTTP_ENABLED.*true" docker-compose.infra.yaml; then
        check_warning "Keycloak HTTP is enabled (should be HTTPS only in production)"
    fi
fi

echo ""
echo "8. Checking database security..."
echo "-----------------------------------"

# Check if database passwords are set
if [ -f .env ]; then
    if grep -q "POSTGRES_PASSWORD=" .env && ! grep -q "POSTGRES_PASSWORD=$" .env; then
        check_issue "PostgreSQL password is configured"
    else
        check_warning "PostgreSQL password may not be set"
    fi
    
    if grep -q "KEYCLOAK_DB_PASSWORD=" .env && ! grep -q "KEYCLOAK_DB_PASSWORD=$" .env; then
        check_issue "Keycloak database password is configured"
    else
        check_warning "Keycloak database password may not be set"
    fi
fi

echo ""
echo "9. Checking firewall configuration..."
echo "-----------------------------------"

# Check if UFW is mentioned in documentation
if [ -f DEPLOYMENT-GUIDE-SIMPLE.md ]; then
    if grep -q "ufw" DEPLOYMENT-GUIDE-SIMPLE.md; then
        check_issue "Firewall configuration is documented"
    else
        check_warning "Firewall configuration may not be documented"
    fi
fi

echo ""
echo "10. Checking backup procedures..."
echo "-----------------------------------"

if [ -f scripts/backup-database.sh ]; then
    check_issue "Database backup script exists"
else
    check_warning "Database backup script not found"
fi

if [ -f runbooks/BACKUP-RECOVERY.md ]; then
    check_issue "Backup and recovery runbook exists"
else
    check_warning "Backup and recovery runbook not found"
fi

echo ""
echo "=========================================="
echo "Security Audit Summary"
echo "=========================================="
echo -e "${GREEN}Passed: $((10 - ISSUES - WARNINGS))${NC}"
echo -e "${YELLOW}Warnings: $WARNINGS${NC}"
echo -e "${RED}Issues: $ISSUES${NC}"
echo ""

if [ $ISSUES -gt 0 ]; then
    echo -e "${RED}Security issues found. Please review and fix.${NC}"
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}Security warnings found. Consider reviewing.${NC}"
    exit 0
else
    echo -e "${GREEN}No security issues found.${NC}"
    exit 0
fi
