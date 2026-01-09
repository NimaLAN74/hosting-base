#!/usr/bin/env python3
"""
Data Coverage Analysis Script
Generates coverage reports for Phase 3 assessment.

This script analyzes:
- Data completeness by country/year
- Coverage gaps in energy types/sectors
- Regional granularity (NUTS levels)
- ML dataset readiness
"""

import os
import sys
import json
from datetime import datetime
from typing import Dict, List, Any
import psycopg2
from psycopg2.extras import RealDictCursor

# Database connection from environment
DB_CONFIG = {
    'host': os.getenv('POSTGRES_HOST', 'localhost'),
    'port': os.getenv('POSTGRES_PORT', '5432'),
    'database': os.getenv('POSTGRES_DB', 'lianel_energy'),
    'user': os.getenv('POSTGRES_USER', 'airflow'),
    'password': os.getenv('POSTGRES_PASSWORD', 'airflow')
}


def get_db_connection():
    """Get database connection."""
    return psycopg2.connect(**DB_CONFIG)


def analyze_country_year_coverage(conn) -> Dict[str, Any]:
    """Analyze data coverage by country and year."""
    with conn.cursor(cursor_factory=RealDictCursor) as cur:
        # Get total records by country and year
        cur.execute("""
            SELECT 
                c.country_code,
                c.country_name,
                e.year,
                COUNT(*) as record_count,
                SUM(e.energy_value_gwh) as total_energy_gwh
            FROM fact_energy_annual e
            JOIN dim_country c ON e.country_code = c.country_code
            GROUP BY c.country_code, c.country_name, e.year
            ORDER BY c.country_code, e.year
        """)
        records = cur.fetchall()
        
        # Get expected countries (EU27)
        cur.execute("""
            SELECT DISTINCT country_code, country_name
            FROM dim_country
            ORDER BY country_code
        """)
        countries = [row['country_code'] for row in cur.fetchall()]
        
        # Get year range
        cur.execute("""
            SELECT MIN(year) as min_year, MAX(year) as max_year
            FROM fact_energy_annual
        """)
        year_range = cur.fetchone()
        
        # Calculate completeness
        expected_years = list(range(year_range['min_year'], year_range['max_year'] + 1))
        expected_combinations = len(countries) * len(expected_years)
        actual_combinations = len(records)
        completeness_pct = (actual_combinations / expected_combinations * 100) if expected_combinations > 0 else 0
        
        # Coverage by country
        country_coverage = {}
        for country_code in countries:
            country_records = [r for r in records if r['country_code'] == country_code]
            country_years = set(r['year'] for r in country_records)
            country_coverage[country_code] = {
                'years_covered': sorted(country_years),
                'years_missing': [y for y in expected_years if y not in country_years],
                'record_count': len(country_records),
                'total_energy_gwh': sum(r['total_energy_gwh'] or 0 for r in country_records)
            }
        
        # Coverage by year
        year_coverage = {}
        for year in expected_years:
            year_records = [r for r in records if r['year'] == year]
            year_countries = set(r['country_code'] for r in year_records)
            year_coverage[year] = {
                'countries_covered': sorted(year_countries),
                'countries_missing': [c for c in countries if c not in year_countries],
                'record_count': len(year_records),
                'total_energy_gwh': sum(r['total_energy_gwh'] or 0 for r in year_records)
            }
        
        return {
            'summary': {
                'total_records': len(records),
                'countries': len(countries),
                'year_range': {'min': year_range['min_year'], 'max': year_range['max_year']},
                'expected_combinations': expected_combinations,
                'actual_combinations': actual_combinations,
                'completeness_percentage': round(completeness_pct, 2)
            },
            'country_coverage': country_coverage,
            'year_coverage': year_coverage,
            'detailed_records': [dict(r) for r in records]
        }


def analyze_energy_product_coverage(conn) -> Dict[str, Any]:
    """Analyze coverage by energy product type."""
    with conn.cursor(cursor_factory=RealDictCursor) as cur:
        cur.execute("""
            SELECT 
                p.product_code,
                p.product_name,
                COUNT(DISTINCT e.country_code) as countries_count,
                COUNT(DISTINCT e.year) as years_count,
                COUNT(*) as record_count,
                SUM(e.energy_value_gwh) as total_energy_gwh
            FROM fact_energy_annual e
            JOIN dim_energy_product p ON e.product_code = p.product_code
            GROUP BY p.product_code, p.product_name
            ORDER BY record_count DESC
        """)
        products = cur.fetchall()
        
        return {
            'total_products': len(products),
            'products': [dict(p) for p in products]
        }


def analyze_energy_flow_coverage(conn) -> Dict[str, Any]:
    """Analyze coverage by energy flow type."""
    with conn.cursor(cursor_factory=RealDictCursor) as cur:
        cur.execute("""
            SELECT 
                f.flow_code,
                f.flow_name,
                COUNT(DISTINCT e.country_code) as countries_count,
                COUNT(DISTINCT e.year) as years_count,
                COUNT(*) as record_count,
                SUM(e.energy_value_gwh) as total_energy_gwh
            FROM fact_energy_annual e
            JOIN dim_energy_flow f ON e.flow_code = f.flow_code
            GROUP BY f.flow_code, f.flow_name
            ORDER BY record_count DESC
        """)
        flows = cur.fetchall()
        
        return {
            'total_flows': len(flows),
            'flows': [dict(f) for f in flows]
        }


def analyze_nuts_coverage(conn) -> Dict[str, Any]:
    """Analyze NUTS regional coverage."""
    with conn.cursor(cursor_factory=RealDictCursor) as cur:
        # Check if dim_region exists and has data
        cur.execute("""
            SELECT 
                level_code,
                COUNT(*) as region_count,
                COUNT(DISTINCT cntr_code) as country_count
            FROM dim_region
            GROUP BY level_code
            ORDER BY level_code
        """)
        nuts_levels = cur.fetchall()
        
        # Check if we can join energy data with regions
        cur.execute("""
            SELECT COUNT(DISTINCT e.country_code) as countries_with_regions
            FROM fact_energy_annual e
            JOIN dim_region r ON e.country_code = r.cntr_code
            WHERE r.level_code = 'NUTS0'
        """)
        join_result = cur.fetchone()
        
        return {
            'nuts_levels': [dict(n) for n in nuts_levels],
            'countries_with_regions': join_result['countries_with_regions'] if join_result else 0,
            'can_join_energy_regions': join_result['countries_with_regions'] > 0 if join_result else False
        }


def analyze_ml_dataset_readiness(conn) -> Dict[str, Any]:
    """Analyze ML dataset readiness."""
    with conn.cursor(cursor_factory=RealDictCursor) as cur:
        # Check if ML dataset table exists
        cur.execute("""
            SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' 
                AND table_name = 'ml_dataset_clustering_v1'
            )
        """)
        table_exists = cur.fetchone()[0]
        
        if not table_exists:
            return {
                'table_exists': False,
                'message': 'ML dataset table not created yet'
            }
        
        # Analyze ML dataset
        cur.execute("""
            SELECT 
                COUNT(*) as total_records,
                COUNT(DISTINCT region_id) as unique_regions,
                COUNT(DISTINCT year) as unique_years,
                COUNT(DISTINCT cntr_code) as unique_countries,
                AVG(total_energy_gwh) as avg_energy,
                AVG(pct_renewable) as avg_renewable_pct,
                AVG(energy_density_gwh_per_km2) as avg_density
            FROM ml_dataset_clustering_v1
        """)
        summary = cur.fetchone()
        
        # Check feature completeness
        cur.execute("""
            SELECT 
                COUNT(*) as total,
                COUNT(total_energy_gwh) as has_energy,
                COUNT(pct_renewable) as has_renewable_pct,
                COUNT(area_km2) as has_area,
                COUNT(energy_density_gwh_per_km2) as has_density
            FROM ml_dataset_clustering_v1
        """)
        features = cur.fetchone()
        
        return {
            'table_exists': True,
            'summary': dict(summary),
            'feature_completeness': {
                'total_records': features['total'],
                'has_energy': features['has_energy'],
                'has_renewable_pct': features['has_renewable_pct'],
                'has_area': features['has_area'],
                'has_density': features['has_density'],
                'completeness_pct': round((features['has_energy'] / features['total'] * 100) if features['total'] > 0 else 0, 2)
            }
        }


def generate_report(analysis_results: Dict[str, Any]) -> str:
    """Generate a human-readable report."""
    report = []
    report.append("=" * 80)
    report.append("DATA COVERAGE ANALYSIS REPORT")
    report.append("=" * 80)
    report.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report.append("")
    
    # Summary
    summary = analysis_results['country_year_coverage']['summary']
    report.append("## SUMMARY")
    report.append(f"Total Records: {summary['total_records']:,}")
    report.append(f"Countries: {summary['countries']}")
    report.append(f"Year Range: {summary['year_range']['min']} - {summary['year_range']['max']}")
    report.append(f"Expected Combinations: {summary['expected_combinations']:,}")
    report.append(f"Actual Combinations: {summary['actual_combinations']:,}")
    report.append(f"Completeness: {summary['completeness_percentage']}%")
    report.append("")
    
    # Country coverage
    report.append("## COUNTRY COVERAGE")
    country_cov = analysis_results['country_year_coverage']['country_coverage']
    for country_code, data in sorted(country_cov.items()):
        missing = len(data['years_missing'])
        total = len(data['years_covered']) + missing
        pct = (len(data['years_covered']) / total * 100) if total > 0 else 0
        report.append(f"{country_code}: {len(data['years_covered'])}/{total} years ({pct:.1f}%) - "
                     f"Missing: {data['years_missing'] if missing <= 5 else f'{missing} years'}")
    report.append("")
    
    # Product coverage
    report.append("## ENERGY PRODUCT COVERAGE")
    products = analysis_results['energy_product_coverage']
    report.append(f"Total Products: {products['total_products']}")
    for p in products['products'][:10]:  # Top 10
        report.append(f"  {p['product_code']}: {p['product_name']} - "
                     f"{p['countries_count']} countries, {p['years_count']} years, "
                     f"{p['record_count']:,} records")
    report.append("")
    
    # Flow coverage
    report.append("## ENERGY FLOW COVERAGE")
    flows = analysis_results['energy_flow_coverage']
    report.append(f"Total Flows: {flows['total_flows']}")
    for f in flows['flows'][:10]:  # Top 10
        report.append(f"  {f['flow_code']}: {f['flow_name']} - "
                     f"{f['countries_count']} countries, {f['years_count']} years, "
                     f"{f['record_count']:,} records")
    report.append("")
    
    # NUTS coverage
    report.append("## NUTS REGIONAL COVERAGE")
    nuts = analysis_results['nuts_coverage']
    for level in nuts['nuts_levels']:
        report.append(f"NUTS{level['level_code']}: {level['region_count']} regions, "
                     f"{level['country_count']} countries")
    report.append(f"Can join energy data with regions: {nuts['can_join_energy_regions']}")
    report.append("")
    
    # ML dataset
    report.append("## ML DATASET READINESS")
    ml = analysis_results['ml_dataset_readiness']
    if ml['table_exists']:
        report.append(f"Table exists: ✅")
        report.append(f"Total records: {ml['summary']['total_records']:,}")
        report.append(f"Unique regions: {ml['summary']['unique_regions']}")
        report.append(f"Unique years: {ml['summary']['unique_years']}")
        report.append(f"Feature completeness: {ml['feature_completeness']['completeness_pct']}%")
    else:
        report.append(f"Table exists: ❌ - {ml['message']}")
    report.append("")
    
    report.append("=" * 80)
    return "\n".join(report)


def main():
    """Main function."""
    print("Starting data coverage analysis...")
    
    try:
        conn = get_db_connection()
        
        print("Analyzing country/year coverage...")
        country_year = analyze_country_year_coverage(conn)
        
        print("Analyzing energy product coverage...")
        products = analyze_energy_product_coverage(conn)
        
        print("Analyzing energy flow coverage...")
        flows = analyze_energy_flow_coverage(conn)
        
        print("Analyzing NUTS coverage...")
        nuts = analyze_nuts_coverage(conn)
        
        print("Analyzing ML dataset readiness...")
        ml = analyze_ml_dataset_readiness(conn)
        
        results = {
            'country_year_coverage': country_year,
            'energy_product_coverage': products,
            'energy_flow_coverage': flows,
            'nuts_coverage': nuts,
            'ml_dataset_readiness': ml
        }
        
        # Generate report
        report = generate_report(results)
        print("\n" + report)
        
        # Save JSON output
        output_file = f"coverage_analysis_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        with open(output_file, 'w') as f:
            json.dump(results, f, indent=2, default=str)
        print(f"\nDetailed results saved to: {output_file}")
        
        # Save report
        report_file = f"coverage_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.txt"
        with open(report_file, 'w') as f:
            f.write(report)
        print(f"Report saved to: {report_file}")
        
        conn.close()
        print("\n✅ Analysis complete!")
        
    except Exception as e:
        print(f"❌ Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
