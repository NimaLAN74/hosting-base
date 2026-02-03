#!/usr/bin/env python3
"""
Eurostat API Testing Script

This script tests the Eurostat API to understand:
- API endpoint structure
- Response format
- Available dimensions
- Data coverage

Usage:
    python3 test-eurostat-api.py
"""

import requests
import json
from typing import Dict, List, Optional
from datetime import datetime

# Eurostat API base URL
BASE_URL = "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data"

# Priority tables to test
PRIORITY_TABLES = {
    "nrg_bal_c": "Complete energy balances",
    "nrg_bal_s": "Simplified energy balances",
    "nrg_cb_e": "Energy supply and consumption",
    "nrg_ind_eff": "Energy efficiency indicators",
    "nrg_ind_ren": "Renewable energy indicators",
}

# EU27 country codes
EU27_COUNTRIES = [
    "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "ES",
    "FI", "FR", "GR", "HR", "HU", "IE", "IT", "LT", "LU",
    "LV", "MT", "NL", "PL", "PT", "RO", "SE", "SI", "SK"
]

# Test years
TEST_YEARS = ["2020", "2021", "2022", "2023"]


def test_table_structure(table_code: str) -> Optional[Dict]:
    """
    Test a Eurostat table to understand its structure.
    
    Args:
        table_code: Eurostat table code (e.g., 'nrg_bal_c')
    
    Returns:
        Dictionary with table structure information
    """
    print(f"\n{'='*60}")
    print(f"Testing table: {table_code}")
    print(f"{'='*60}")
    
    # First, get table structure without filters
    try:
        response = requests.get(f"{BASE_URL}/{table_code}?format=JSON", timeout=30)
        response.raise_for_status()
        data = response.json()
        
        # Extract structure
        dimensions = data.get("dimension", {})
        value_count = len(data.get("value", {}))
        
        print(f"✅ Table accessible")
        print(f"   Dimensions: {list(dimensions.keys())}")
        print(f"   Total values: {value_count:,}")
        
        # Show dimension details
        print(f"\n   Dimension details:")
        for dim_name, dim_data in dimensions.items():
            if isinstance(dim_data, dict) and "category" in dim_data:
                categories = dim_data["category"].get("index", {})
                print(f"     - {dim_name}: {len(categories)} categories")
                # Show sample categories
                sample = list(categories.keys())[:5]
                if sample:
                    print(f"       Sample: {', '.join(sample)}")
        
        return {
            "table_code": table_code,
            "dimensions": list(dimensions.keys()),
            "value_count": value_count,
            "dimension_details": {
                name: len(dim.get("category", {}).get("index", {}))
                for name, dim in dimensions.items()
            }
        }
        
    except requests.exceptions.RequestException as e:
        print(f"❌ Error accessing table: {e}")
        return None


def test_table_with_filters(table_code: str, country: str = "SE", years: List[str] = None) -> Optional[Dict]:
    """
    Test a Eurostat table with specific filters.
    
    Args:
        table_code: Eurostat table code
        country: Country code (ISO 2-letter)
        years: List of years to query
    
    Returns:
        Dictionary with filtered data information
    """
    if years is None:
        years = TEST_YEARS
    
    print(f"\n   Testing with filters: country={country}, years={years}")
    
    # Build query parameters
    params = {
        "format": "JSON",
        "geo": country,
    }
    
    # Add time parameters
    for year in years:
        params[f"time"] = year
    
    try:
        response = requests.get(f"{BASE_URL}/{table_code}", params=params, timeout=30)
        response.raise_for_status()
        data = response.json()
        
        values = data.get("value", {})
        print(f"   ✅ Filtered query successful")
        print(f"      Values returned: {len(values):,}")
        
        # Show sample values
        if values:
            sample = dict(list(values.items())[:3])
            print(f"      Sample values: {sample}")
        
        return {
            "country": country,
            "years": years,
            "value_count": len(values),
            "sample_values": dict(list(values.items())[:3]) if values else {}
        }
        
    except requests.exceptions.RequestException as e:
        print(f"   ❌ Error with filtered query: {e}")
        return None


def test_coverage(table_code: str, countries: List[str] = None, years: List[str] = None) -> Dict:
    """
    Test data coverage for multiple countries.
    
    Args:
        table_code: Eurostat table code
        countries: List of country codes to test
        years: List of years to test
    
    Returns:
        Coverage analysis dictionary
    """
    if countries is None:
        countries = EU27_COUNTRIES[:5]  # Test first 5 for speed
    if years is None:
        years = TEST_YEARS
    
    print(f"\n   Testing coverage for {len(countries)} countries...")
    
    coverage = {}
    for country in countries:
        result = test_table_with_filters(table_code, country, years)
        if result:
            coverage[country] = {
                "values": result["value_count"],
                "status": "✅" if result["value_count"] > 0 else "❌"
            }
        else:
            coverage[country] = {"status": "❌", "values": 0}
    
    return coverage


def main():
    """Main function to run all tests."""
    print("="*60)
    print("Eurostat API Testing Script")
    print(f"Date: {datetime.now().strftime('%d/%m/%Y %H:%M:%S')}")
    print("="*60)
    
    results = {}
    
    # Test each priority table
    for table_code, description in PRIORITY_TABLES.items():
        print(f"\n📊 {description} ({table_code})")
        
        # Test table structure
        structure = test_table_structure(table_code)
        if structure:
            results[table_code] = {
                "description": description,
                "structure": structure
            }
            
            # Test with filters (Sweden as example)
            filtered = test_table_with_filters(table_code, "SE", TEST_YEARS)
            if filtered:
                results[table_code]["filtered_test"] = filtered
            
            # Test coverage for a few countries
            coverage = test_coverage(table_code, EU27_COUNTRIES[:3], TEST_YEARS)
            results[table_code]["coverage"] = coverage
    
    # Summary
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)
    
    for table_code, result in results.items():
        print(f"\n{table_code}: {result['description']}")
        if "structure" in result:
            print(f"  Dimensions: {', '.join(result['structure']['dimensions'])}")
        if "filtered_test" in result:
            print(f"  Test query (SE, 2020-2023): {result['filtered_test']['value_count']} values")
    
    # Save results to JSON
    output_file = "eurostat-api-test-results.json"
    with open(output_file, "w") as f:
        json.dump(results, f, indent=2)
    
    print(f"\n✅ Results saved to: {output_file}")
    print("\nNext steps:")
    print("1. Review the JSON output file")
    print("2. Document findings in 02-data-inventory/01-eurostat-inventory.md")
    print("3. Create sample API requests for each table")


if __name__ == "__main__":
    main()

