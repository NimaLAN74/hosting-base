# D1 — Eurostat Energy Data Inventory  
Version 1.0

## 1. Purpose
Inventory and describe Eurostat datasets relevant for EU energy consumption, production, transformation and indicators.

## 2. Main Dataset Families
- `nrg_bal_c`: complete energy balances (primary macro dataset)
- Simplified balances and supply/transformation tables
- Final energy consumption by sector and product
- Energy indicators (renewable share, efficiency, SDG7)
- Electricity and heat production and trade

## 3. Key Dimensions
- GEO (country)
- TIME (year)
- NRG_BAL (flow)
- PRODUCT (energy product)
- SECTOR (where applicable)
- UNIT (ktoe, TJ, GWh)
- VALUE (numeric)

## 4. Time Coverage
- Balances: ~1960s → latest full year
- Final consumption and indicators: generally from 1990s onward

## 5. Data Quality
- Sporadic gaps in early years for some countries
- Methodological updates over time (products and flows)
- Mixed units across tables (harmonised later via D6)

## 6. Use in Platform
- Macro baseline for energy mix and sectoral patterns
- Long-term trend and scenario analysis
- Annual features for regional clustering and ML models
