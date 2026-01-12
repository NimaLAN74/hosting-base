# Jupyter Notebooks - Energy Data Analysis

This directory contains Jupyter notebooks for data analysis and quality verification.

## Notebooks

### 01-data-quality-analysis.ipynb
**Purpose**: Comprehensive data quality verification, focusing on renewable energy data issues

**Contents**:
- Database connection setup
- Renewable energy issue investigation
- Product code analysis (identifies missing fossil products)
- Data quality report generation
- Recommendations for fixing issues

**Key Findings**:
- All countries show 100% renewable energy (SUSPICIOUS)
- Fossil product codes (C0110, C0121, C0350) are missing from `fact_energy_annual`
- Root cause: Fossil products are not being ingested from Eurostat API

**Usage**:
```bash
# Run in Jupyter environment
jupyter notebook 01-data-quality-analysis.ipynb
```

## Setup

### Prerequisites
- Python 3.9+
- Jupyter Notebook or JupyterLab
- Required packages:
  ```bash
  pip install pandas numpy matplotlib seaborn sqlalchemy psycopg2-binary
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
1. **Missing Fossil Products**: C0110, C0121, C0350 not in `fact_energy_annual`
2. **100% Renewable Energy**: All countries show 100% renewable (data quality issue)
3. **Zero Fossil Energy**: All countries have 0 GWh fossil energy

### Next Steps
1. Investigate Eurostat API response for fossil product codes
2. Review ingestion DAG filtering logic
3. Verify product code mapping
4. Re-run ingestion and harmonization DAGs

## Notes

- Update database password in notebooks before running
- Notebooks assume database is accessible from the execution environment
- Some queries may take time depending on data volume
