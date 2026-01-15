# Incident Response Procedures
**Last Updated**: January 15, 2026  
**Owner**: Operations Team

---

## Overview

This document defines procedures for responding to incidents in the Lianel platform.

---

## 1. Incident Classification

### 1.1 Severity Levels

#### Critical (P1)
- **Definition**: Complete system outage, data loss, security breach
- **Response Time**: Immediate (<15 minutes)
- **Resolution Target**: <4 hours
- **Examples**:
  - Complete service unavailability
  - Data corruption or loss
  - Security breach
  - Database failure

#### High (P2)
- **Definition**: Major functionality impacted, significant user impact
- **Response Time**: <1 hour
- **Resolution Target**: <8 hours
- **Examples**:
  - Partial service outage
  - Critical DAG failures
  - API performance degradation
  - Authentication failures

#### Medium (P3)
- **Definition**: Limited functionality impacted, minor user impact
- **Response Time**: <4 hours
- **Resolution Target**: <24 hours
- **Examples**:
  - Single DAG failure
  - Non-critical feature issues
  - Performance degradation (non-critical)
  - Data quality issues

#### Low (P4)
- **Definition**: Minor issues, minimal user impact
- **Response Time**: <1 business day
- **Resolution Target**: <1 week
- **Examples**:
  - Cosmetic issues
  - Documentation errors
  - Minor performance issues
  - Non-critical alerts

---

## 2. Incident Detection

### 2.1 Detection Sources

#### Automated Monitoring
- Prometheus alerts
- Grafana dashboards
- Health checks
- Log aggregation (Loki)

#### Manual Detection
- User reports
- Support tickets
- Manual testing
- System checks

---

### 2.2 Alert Channels

#### Critical Alerts
- Prometheus → Alert Manager
- Email notifications
- On-call pager
- Slack notifications

#### Monitoring Dashboards
- System Health: `https://monitoring.lianel.se/d/system-health`
- Pipeline Status: `https://monitoring.lianel.se/d/pipeline-status`
- Error Tracking: `https://monitoring.lianel.se/d/error-tracking`
- SLA Monitoring: `https://monitoring.lianel.se/d/sla-monitoring`

---

## 3. Incident Response Process

### 3.1 Initial Response

#### Step 1: Acknowledge
- Acknowledge incident
- Assign incident owner
- Create incident ticket
- Notify stakeholders

#### Step 2: Assess
- Determine severity
- Identify affected systems
- Assess user impact
- Check monitoring dashboards

#### Step 3: Contain
- Isolate affected systems
- Stop propagation
- Implement workarounds
- Preserve evidence

---

### 3.2 Investigation

#### Gather Information
```bash
# Check system status
docker ps --format 'table {{.Names}}\t{{.Status}}'

# Check service health
curl http://localhost:3001/health
curl http://localhost:3000/health
curl http://localhost:8974/health

# Check logs
docker logs <service> --tail 100

# Check metrics
# Access Grafana dashboards
```

#### Review Runbooks
- DAG Failure Recovery
- Service Restart Procedures
- Database Maintenance
- Data Quality Issues
- Performance Troubleshooting

---

### 3.3 Resolution

#### Implement Fix
- Follow appropriate runbook
- Apply fixes carefully
- Test before full deployment
- Document changes

#### Verify Resolution
- Test affected functionality
- Check monitoring dashboards
- Verify user impact resolved
- Monitor for recurrence

---

### 3.4 Communication

#### During Incident
- **Internal**: Update incident ticket, notify team
- **Stakeholders**: Provide status updates every 30-60 minutes
- **Users**: Post status page updates if public-facing

#### After Resolution
- **Incident Report**: Document root cause, resolution, prevention
- **Post-Mortem**: Schedule if P1/P2 incident
- **Lessons Learned**: Share with team

---

## 4. Escalation Procedures

### 4.1 Escalation Triggers

#### Escalate to Level 2 (Development Team)
- Code/config issues
- Bug fixes needed
- Feature problems
- After 1 hour of troubleshooting

#### Escalate to Level 3 (Infrastructure Team)
- Infrastructure issues
- Network problems
- Hardware failures
- After 2 hours of troubleshooting

#### Escalate to Level 4 (Management)
- Business impact
- Extended outage (>4 hours)
- Security incidents
- Customer escalations

---

### 4.2 Escalation Path

```
Level 1: Operations Team
    ↓ (if unresolved after 1 hour)
Level 2: Development Team
    ↓ (if unresolved after 2 hours)
Level 3: Infrastructure Team
    ↓ (if unresolved after 4 hours)
Level 4: Management
```

---

## 5. Common Incident Scenarios

### 5.1 Service Outage

#### Symptoms
- Service unavailable
- 502/503 errors
- Health checks failing

#### Response
1. Check service status
2. Review service logs
3. Check dependencies
4. Restart service if needed
5. Verify recovery

#### Runbook
- See: `SERVICE-RESTART-PROCEDURES.md`

---

### 5.2 DAG Failures

#### Symptoms
- Multiple DAGs failing
- Tasks stuck
- High failure rate

#### Response
1. Check DAG status
2. Review task logs
3. Check system resources
4. Clear and re-run if needed
5. Investigate root cause

#### Runbook
- See: `DAG-FAILURE-RECOVERY.md`

---

### 5.3 Database Issues

#### Symptoms
- Connection failures
- Slow queries
- Database errors

#### Response
1. Check database status
2. Review connection count
3. Check for locks
4. Review slow queries
5. Optimize or restart

#### Runbook
- See: `DATABASE-MAINTENANCE.md`

---

### 5.4 Performance Degradation

#### Symptoms
- Slow API responses
- High CPU/memory
- Timeout errors

#### Response
1. Check system resources
2. Review performance metrics
3. Identify bottlenecks
4. Optimize or scale
5. Monitor improvements

#### Runbook
- See: `PERFORMANCE-TROUBLESHOOTING.md`

---

### 5.5 Data Quality Issues

#### Symptoms
- Missing data
- Stale data
- Data errors

#### Response
1. Check data quality metrics
2. Review ingestion logs
3. Identify affected data
4. Re-run ingestion if needed
5. Verify data quality

#### Runbook
- See: `DATA-QUALITY-ISSUES.md`

---

## 6. Post-Incident Review

### 6.1 Incident Report Template

```markdown
# Incident Report: [Title]

## Incident Details
- **Date**: [Date]
- **Time**: [Start time] - [End time]
- **Duration**: [Duration]
- **Severity**: [P1/P2/P3/P4]
- **Incident Owner**: [Name]

## Impact
- **Affected Systems**: [List]
- **User Impact**: [Description]
- **Business Impact**: [Description]

## Timeline
- [Time] - Incident detected
- [Time] - Investigation started
- [Time] - Root cause identified
- [Time] - Resolution implemented
- [Time] - Incident resolved

## Root Cause
[Detailed description]

## Resolution
[Steps taken to resolve]

## Prevention
- [Action item 1]
- [Action item 2]
- [Action item 3]

## Lessons Learned
[Key takeaways]
```

---

### 6.2 Post-Mortem Process

#### When to Conduct
- P1 incidents (always)
- P2 incidents (if significant impact)
- Recurring incidents
- Complex incidents

#### Process
1. **Schedule**: Within 48 hours of resolution
2. **Participants**: Incident responders, stakeholders
3. **Agenda**:
   - Timeline review
   - Root cause analysis
   - What went well
   - What could be improved
   - Action items

#### Action Items
- Update runbooks
- Improve monitoring
- Fix root causes
- Update documentation
- Training needs

---

## 7. Communication Templates

### 7.1 Incident Notification

```
Subject: [SEVERITY] Incident: [Title]

Incident Details:
- Severity: [P1/P2/P3/P4]
- Status: [Investigating/Resolved]
- Affected Systems: [List]
- User Impact: [Description]

Current Status:
[Brief update]

Next Update: [Time]
```

---

### 7.2 Status Update

```
Subject: Incident Update: [Title]

Status: [Investigating/Resolved]

Update:
[Current status and progress]

ETA: [If known]
```

---

### 7.3 Resolution Notification

```
Subject: Incident Resolved: [Title]

Status: Resolved

Resolution:
[What was done to resolve]

Prevention:
[Steps taken to prevent recurrence]

Incident Report: [Link]
```

---

## 8. Quick Reference

### Incident Response Checklist
- [ ] Acknowledge incident
- [ ] Classify severity
- [ ] Assign owner
- [ ] Notify stakeholders
- [ ] Investigate root cause
- [ ] Implement fix
- [ ] Verify resolution
- [ ] Document incident
- [ ] Schedule post-mortem (if needed)

### Key Contacts
- **Operations**: [Contact]
- **Development**: [Contact]
- **Infrastructure**: [Contact]
- **On-Call**: [Contact]
- **Management**: [Contact]

### Key Links
- Monitoring: `https://monitoring.lianel.se`
- Airflow: `https://airflow.lianel.se`
- Runbooks: `/root/lianel/dc/runbooks/`

---

**Status**: Active  
**Review Frequency**: Quarterly  
**Last Review**: January 15, 2026
