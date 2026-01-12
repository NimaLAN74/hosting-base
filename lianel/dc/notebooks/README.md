# Jupyter Notebooks - Energy Data Analysis

This directory contains Jupyter notebooks for data analysis and quality verification.

## ⚠️ IMPORTANT: Workflow & Data Quality

**CRITICAL**: Always verify data quality before performing analysis. Follow this workflow:

1. **01-data-quality-analysis.ipynb** - Run FIRST to identify data quality issues
2. **02-exploratory-data-analysis.ipynb** - Explore data patterns and anomalies
3. **03-bias-detection.ipynb** - Detect systematic biases and missing patterns
4. **04-ml-feature-analysis.ipynb** - Analyze ML features (filters to `year >= 2018` by default)
5. **05-trend-analysis.ipynb** - Trend analysis (includes data quality checks)

**Data Quality Requirements**:
- Verify fossil energy data completeness (2016-2017 had issues)
- Check for suspicious renewable percentages (not all 100%)
- Identify extreme YoY changes that indicate data errors
- Filter incomplete years before analysis
- Document any data quality issues found

## Notebooks

### 01-data-quality-analysis.ipynb
**Purpose**: Comprehensive data quality verification, focusing on renewable energy data issues

**Contents**:
- Database connection setup
- Renewable energy issue investigation
- Product code analysis (identifies missing fossil products)
- Data quality report generation
- Root cause analysis (Eurostat API investigation)
- Recommendations for fixing issues

**Key Findings**:
- 2016-2017: Only renewable products ingested (missing fossil data)
- 2018+: Both renewable and fossil products ingested
- Root cause: Incomplete data ingestion in early years

---

### 02-exploratory-data-analysis.ipynb
**Purpose**: Comprehensive exploratory analysis of energy data to identify patterns, anomalies, and data quality issues

**Contents**:
- Load ML dataset data
- Investigate 2017-2018 anomalies (YoY changes, renewable percentages)
- Root cause analysis: Data completeness by year
- Data distribution analysis
- Correlation analysis
- Key findings and recommendations

**Key Findings**:
- 2018 YoY spikes (2000%+) due to incomplete 2017 data
- 2017 shows 100% renewable due to missing fossil data
- Recommendations for data quality flags and re-ingestion

---

### 03-bias-detection.ipynb
**Purpose**: Identify data bias, missing patterns, outliers, and systematic data quality issues

**Contents**:
- Temporal bias detection (missing years, incomplete periods)
- Geographic bias detection (missing countries, regions)
- Outlier detection using IQR and Z-score methods
- Data completeness analysis
- Summary and recommendations

**Key Findings**:
- Temporal bias: Missing years for some countries
- Data completeness bias: 2016-2017 missing fossil data
- Geographic bias: Potential missing countries
- Outlier bias: Extreme values that may skew analysis

---

### 04-ml-feature-analysis.ipynb
**Purpose**: Analyze ML dataset features, feature importance, distributions, and relationships

**Contents**:
- Load and explore ML dataset features
- Feature statistics and distributions
- Feature correlations and relationships
- Feature importance analysis
- Feature quality assessment

**Key Features Analyzed**:
- Target variables (total, renewable, fossil energy)
- Time features (year_index, lags, trends)
- YoY changes and rolling statistics
- Spatial features (area, energy density)
- Percentage features

---

### 05-trend-analysis.ipynb
**Purpose**: Analyze time series trends, forecasting patterns, and seasonal analysis

**Contents**:
- **Data Quality Verification** (CRITICAL - runs first)
  - Missing value checks
  - Fossil energy data completeness by year
  - Renewable percentage validation
  - Extreme YoY change detection
  - Country coverage verification
- **Data Filtering & Preparation**
  - Filter incomplete years (<90% fossil data completeness)
  - Remove extreme outliers (>200% YoY changes)
- Load time series data
- Overall trend analysis (total, renewable, fossil energy)
- Country-level trend analysis
- Forecasting pattern analysis

**Key Metrics**:
- Linear trend slopes and R² values
- Renewable energy transition patterns
- Growth rates by country
- Trend strength indicators

**Note**: This notebook now includes comprehensive data quality checks before analysis to ensure reliable results.

## Setup

### Prerequisites
- Python 3.9+
- Jupyter Notebook or JupyterLab
- Required packages:
  ```bash
  pip install pandas numpy matplotlib seaborn sqlalchemy psycopg2-binary scipy scikit-learn
  ```

### Database Connection
The notebooks connect to the PostgreSQL database using:
- Host: `172.18.0.1`
- Database: `lianel_energy`
- User: `airflow`
- Password: From environment variable (update in notebook)

## Running Notebooks

### Option 1: Local Jupyter
```bash
cd lianel/dc/notebooks
jupyter notebook
```

### Option 2: Docker Container
```bash
# Run Jupyter in Airflow worker container
docker exec -it dc-airflow-worker-1 bash
jupyter notebook --ip=0.0.0.0 --port=8888 --no-browser --allow-root
```

### Option 3: VS Code
Open the `.ipynb` files directly in VS Code (Jupyter extension required)

## Data Quality Issues

### Current Issues
1. **2016-2017 Incomplete Data**: Only renewable products ingested, missing fossil data
2. **2018 YoY Anomalies**: Extreme spikes (2000%+) due to incomplete baseline data
3. **2017 Renewable %**: Shows 100% renewable due to missing fossil energy data

### Recommendations
1. **Data Quality Flags**: Add completeness flags to ML datasets
2. **Filter Incomplete Years**: Exclude 2016-2017 from YoY calculations or mark as incomplete
3. **Re-ingest 2016-2017**: Re-run ingestion DAGs to capture fossil products
4. **Dashboard Warnings**: Add warnings in Grafana for incomplete data periods

## Best Practices

1. **Always run data quality checks first** - Don't skip notebook 01
2. **Review quality warnings** - Address issues before proceeding
3. **Filter incomplete data** - Exclude years with <90% fossil data completeness
4. **Document findings** - Note any data quality issues in your analysis
5. **Re-run after data fixes** - If you fix data issues, re-run quality checks

## Notes

- Update database password in notebooks before running
- Notebooks assume database is accessible from the execution environment
- Some queries may take time depending on data volume
- Notebook 05 now includes built-in data quality verification and filtering