# D6 — Unit Conversions  
Version 1.0

## 1. Canonical Units
- Energy: GWh
- Power: MW
- Area: km²

## 2. Energy Conversion Formulas
- ktoe → GWh: `GWh = ktoe × 11.63`
- TJ → GWh: `GWh = TJ / 3.6`
- MWh → GWh: `GWh = MWh / 1000`
- GWh → ktoe: `ktoe = GWh / 11.63`

## 3. Power Conversion
- MW → kW: `kW = MW × 1000`
- MW → GW: `GW = MW / 1000`

## 4. Area Conversion
- m² → km²: `km² = m² / 1e6`
- hectares → km²: `km² = hectares / 100`

## 5. Combined Logic
- Eurostat macro: ktoe or TJ converted to GWh
- ENTSO-E: MW × hours → MWh → GWh
