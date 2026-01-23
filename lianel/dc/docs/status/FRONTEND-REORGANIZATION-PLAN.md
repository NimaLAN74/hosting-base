# Frontend Reorganization Plan

## Overview
Reorganizing the web portal to be more user-friendly with main functions visible on the main page, analytics/visualizations prominently displayed, and better categorization.

## Changes Made

### 1. Grafana Configuration ✅
- **Fixed**: Added `allow_embedding = true` to enable iframe embedding
- **Fixed**: Users with admin/editor roles can now see configuration options
- **Status**: Grafana restarted with new configuration

### 2. New Monitoring Page ✅
- **Created**: `/monitoring` route with Grafana dashboard integration
- **Features**:
  - List of all Grafana dashboards organized by category
  - Direct links to open dashboards in Grafana
  - Embedded dashboard viewer (iframe) with modal
  - Categories: Infrastructure, Data Pipelines, Operations, Data Quality, Analytics

### 3. Dashboard Main Page Reorganization ✅
- **Main Analytics Section**: Prominently displays:
  - EU Energy Data
  - Electricity Timeseries
  - Geospatial Features
  - Monitoring & Dashboards
- **Other Services Section**: Moved less-frequently-used services to separate section
- **Better Organization**: Clear separation between analytics/visualizations and other services

## Dashboard Structure

### Main Page (Dashboard.js)
1. **Welcome Section** - Personalized greeting
2. **Analytics & Data Visualizations** - Main functions (prominent)
   - EU Energy Data
   - Electricity Timeseries
   - Geospatial Features
   - Monitoring & Dashboards
3. **Other Services** - Secondary services
   - User Profile
   - Admin services (if admin)
4. **Recent Activity** - Activity feed
5. **Quick Stats** - System statistics

### Monitoring Page (Monitoring.js)
- **Categories**:
  - Infrastructure (System Health, Database Performance)
  - Data Pipelines (Pipeline Status)
  - Operations (Error Tracking, SLA Monitoring)
  - Data Quality (Data Quality)
  - Analytics (Energy & OSM Features, Web Analytics, Energy Metrics)
- **Features**:
  - Dashboard cards with descriptions
  - "View Embedded" button - opens dashboard in modal iframe
  - "Open in Grafana" button - opens dashboard in new tab
  - Full-screen modal for embedded dashboards

## Grafana Dashboard Integration

### Iframe Embedding
- **URL Format**: `https://monitoring.lianel.se/d/{uid}?kiosk=tv&orgId=1`
- **Kiosk Mode**: `kiosk=tv` hides Grafana UI for cleaner embedding
- **Configuration**: `allow_embedding = true` in grafana.ini

### Dashboard UIDs
All dashboards use their JSON filename as UID:
- `system-health`
- `pipeline-status`
- `database-performance`
- `error-tracking`
- `sla-monitoring`
- `data-quality`
- `energy-osm-features`
- `web-analytics`
- `energy-metrics`

## Routes

```
/                    → Dashboard (main page with analytics)
/profile             → User Profile
/energy              → EU Energy Data
/electricity         → Electricity Timeseries
/geo                 → Geospatial Features
/monitoring          → Monitoring & Dashboards (NEW)
/admin/users         → User Management (admin only)
```

## Next Steps

1. **Test Monitoring Page**:
   - Verify all dashboard links work
   - Test iframe embedding
   - Check authentication flow

2. **Grafana Authentication**:
   - May need to configure anonymous access or token-based auth for iframes
   - Or use proxy authentication

3. **Additional Improvements**:
   - Add search/filter to monitoring page
   - Add favorites/bookmarks
   - Add dashboard preview thumbnails

## Files Created/Modified

### New Files:
- `frontend/src/monitoring/Monitoring.js` - Monitoring page component
- `frontend/src/monitoring/Monitoring.css` - Monitoring page styles

### Modified Files:
- `frontend/src/App.js` - Added `/monitoring` route
- `frontend/src/Dashboard.js` - Reorganized to show analytics prominently
- `monitoring/grafana/grafana.ini` - Added `allow_embedding = true`

## Status
✅ **Complete** - Frontend reorganized, Monitoring page created, Grafana embedding enabled
