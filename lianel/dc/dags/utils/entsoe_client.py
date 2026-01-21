"""
ENTSO-E Transparency Platform API Client

This module provides a Python client for accessing ENTSO-E Transparency Platform data.
ENTSO-E provides high-frequency electricity data (load, generation) for European countries.

API Documentation: https://transparency.entsoe.eu/
API Base URL: https://web-api.tp.entsoe.eu/api

Note: ENTSO-E API may require registration and authentication token.
"""

import requests
import xml.etree.ElementTree as ET
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Any
import time
import logging

logger = logging.getLogger(__name__)

# ENTSO-E API Configuration
ENTSOE_BASE_URL = "https://web-api.tp.entsoe.eu/api"
ENTSOE_DOMAIN = "10Y1001A1001A82H"  # European domain code

# Production type mapping (ENTSO-E codes to our codes)
PRODUCTION_TYPE_MAP = {
    'B01': 'B01',  # Biomass
    'B02': 'B02',  # Fossil Brown coal/Lignite
    'B03': 'B03',  # Fossil Coal-derived gas
    'B04': 'B04',  # Fossil Gas
    'B05': 'B05',  # Fossil Hard coal
    'B06': 'B06',  # Fossil Oil
    'B09': 'B09',  # Geothermal
    'B10': 'B10',  # Hydro Pumped Storage
    'B11': 'B11',  # Hydro Run-of-river and poundage
    'B12': 'B12',  # Hydro Water Reservoir
    'B13': 'B13',  # Marine
    'B14': 'B14',  # Nuclear
    'B15': 'B15',  # Other renewable
    'B16': 'B16',  # Solar
    'B17': 'B17',  # Waste
    'B18': 'B18',  # Wind Offshore
    'B19': 'B19',  # Wind Onshore
    'B20': 'B20',  # Other
}

# Country code to bidding zone mapping
# Note: Some countries have multiple bidding zones (e.g., SE1, SE2, SE3, SE4 for Sweden)
COUNTRY_BIDDING_ZONES = {
    'AT': ['10YAT-APG------L'],  # Austria
    'BE': ['10YBE----------2'],  # Belgium
    'BG': ['10YCA-BULGARIA-R'],  # Bulgaria
    'HR': ['10YHR-HE------C'],  # Croatia
    'CZ': ['10YCZ-CEPS-----N'],  # Czech Republic
    'DK': ['10Y1001A1001A65H', '10Y1001A1001A796'],  # Denmark (DK1, DK2)
    'EE': ['10Y1001A1001A39I'],  # Estonia
    'FI': ['10YFI-1--------U'],  # Finland
    'FR': ['10YFR-RTE------C'],  # France
    'DE': ['10Y1001A1001A83F'],  # Germany
    'EL': ['10YGR-HTSO-----Y'],  # Greece
    'HU': ['10YHU-MAV------1'],  # Hungary
    'IE': ['10YIE-1001A00010'],  # Ireland
    'IT': ['10YIT-GRTN-----B'],  # Italy
    'LV': ['10YLV-1001A00074'],  # Latvia
    'LT': ['10YLT-1001A0008Q'],  # Lithuania
    'LU': ['10YLU-CEGEDEL-NQ'],  # Luxembourg
    'NL': ['10YNL----------L'],  # Netherlands
    'PL': ['10YPL-AREA-----S'],  # Poland
    'PT': ['10YPT-REN------W'],  # Portugal
    'RO': ['10YRO-TEL------P'],  # Romania
    'SK': ['10YSK-SEPS-----K'],  # Slovakia
    'SI': ['10YSI-ELES-----O'],  # Slovenia
    'ES': ['10YES-REE------0'],  # Spain
    'SE': ['10YSE-1--------K', '10Y1001A1001A44P', '10Y1001A1001A45N', '10Y1001A1001A46L'],  # Sweden (SE1, SE2, SE3, SE4)
}


class ENTSOEClient:
    """
    Client for ENTSO-E Transparency Platform API.
    
    Usage:
        client = ENTSOEClient(api_token="your-token")
        data = client.get_load_data(country_code="SE", start_date="2024-01-01", end_date="2024-01-02")
    """
    
    def __init__(self, api_token: Optional[str] = None, base_url: str = ENTSOE_BASE_URL):
        """
        Initialize ENTSO-E client.
        
        Args:
            api_token: API token (if required). Can be None for public endpoints.
            base_url: Base URL for ENTSO-E API
        """
        self.api_token = api_token
        self.base_url = base_url
        self.session = requests.Session()
        # ENTSO-E API uses securityToken query parameter, not Authorization header
        # Don't set Authorization header as it may cause 401 errors
        self.session.headers.update({'Content-Type': 'application/xml'})
    
    def _make_request(self, params: Dict[str, Any], retries: int = 3) -> Optional[ET.Element]:
        """
        Make request to ENTSO-E API and parse XML response.
        
        Args:
            params: Query parameters for the API request
            retries: Number of retry attempts
            
        Returns:
            Parsed XML ElementTree root, or None if request failed
        """
        url = f"{self.base_url}"
        
        for attempt in range(retries):
            try:
                response = self.session.get(url, params=params, timeout=30)
                response.raise_for_status()
                
                # Parse XML response
                root = ET.fromstring(response.content)
                
                # Check for errors in response
                if root.tag == 'Acknowledgement_MarketDocument':
                    # Check for rejection reason
                    reason = root.find('.//{*}reason')
                    if reason is not None:
                        code = reason.find('.//{*}code')
                        text = reason.find('.//{*}text')
                        if code is not None and code.text != '0':
                            error_msg = text.text if text is not None else 'Unknown error'
                            logger.warning(f"ENTSO-E API error: {error_msg}")
                            return None
                
                return root
                
            except requests.exceptions.RequestException as e:
                logger.warning(f"ENTSO-E API request failed (attempt {attempt + 1}/{retries}): {e}")
                if attempt < retries - 1:
                    time.sleep(2 ** attempt)  # Exponential backoff
                else:
                    logger.error(f"ENTSO-E API request failed after {retries} attempts")
                    return None
            except ET.ParseError as e:
                logger.error(f"Failed to parse ENTSO-E XML response: {e}")
                return None
        
        return None
    
    def get_load_data(
        self, 
        country_code: str, 
        start_date: str, 
        end_date: str,
        bidding_zone: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """
        Get actual load data for a country.
        
        Args:
            country_code: ISO 2-letter country code
            start_date: Start date in format 'YYYY-MM-DD' or 'YYYY-MM-DDTHH:MM'
            end_date: End date in format 'YYYY-MM-DD' or 'YYYY-MM-DDTHH:MM'
            bidding_zone: Optional bidding zone code (if None, uses first zone for country)
            
        Returns:
            List of load data records
        """
        # Get bidding zone for country
        if bidding_zone is None:
            zones = COUNTRY_BIDDING_ZONES.get(country_code, [])
            if not zones:
                logger.warning(f"No bidding zones found for country {country_code}")
                return []
            bidding_zone = zones[0]  # Use first zone
        
        # Build request parameters
        # ENTSO-E API requires YYYYMMDDHHMM format (12 digits)
        # Convert date strings to proper format
        def format_date_for_api(date_str: str, is_end: bool = False) -> str:
            """Format date string to ENTSO-E API format YYYYMMDDHHMM."""
            # Remove separators
            clean_date = date_str.replace('-', '').replace(':', '').replace('T', '').replace(' ', '')
            # If only date (8 chars), add time: 0000 for start, 2300 for end
            if len(clean_date) == 8:
                if is_end:
                    return clean_date + '2300'  # End of day
                else:
                    return clean_date + '0000'  # Start of day
            # If already has time, pad to 12 chars if needed
            elif len(clean_date) < 12:
                clean_date = clean_date.ljust(12, '0')
            return clean_date[:12]  # Ensure exactly 12 digits
        
        params = {
            'securityToken': self.api_token or '',
            'documentType': 'A65',  # Actual Total Load
            'processType': 'A16',   # Realised
            'outBiddingZone_Domain': bidding_zone,
            'periodStart': format_date_for_api(start_date, is_end=False),
            'periodEnd': format_date_for_api(end_date, is_end=True),
        }
        
        root = self._make_request(params)
        if root is None:
            return []
        
        # Parse load data from XML
        records = []
        time_series = root.findall('.//{*}TimeSeries')
        
        for ts in time_series:
            period = ts.find('.//{*}Period')
            if period is None:
                continue
            
            resolution = period.find('.//{*}resolution')
            resolution_text = resolution.text if resolution is not None else 'PT60M'
            
            points = period.findall('.//{*}Point')
            for point in points:
                position = point.find('.//{*}position')
                quantity = point.find('.//{*}quantity')
                
                if position is None or quantity is None:
                    continue
                
                # Calculate timestamp from start time and position
                start_time = period.find('.//{*}start')
                if start_time is None:
                    continue
                
                start_dt = datetime.fromisoformat(start_time.text.replace('Z', '+00:00'))
                
                # Position is 1-indexed, so subtract 1
                position_num = int(position.text) - 1
                
                # Calculate actual timestamp based on resolution
                if resolution_text == 'PT60M' or resolution_text == 'PT1H':
                    timestamp = start_dt + timedelta(hours=position_num)
                elif resolution_text == 'PT15M':
                    timestamp = start_dt + timedelta(minutes=15 * position_num)
                else:
                    # Default to hourly
                    timestamp = start_dt + timedelta(hours=position_num)
                
                records.append({
                    'timestamp_utc': timestamp.isoformat(),
                    'country_code': country_code,
                    'bidding_zone': bidding_zone,
                    'load_mw': float(quantity.text),
                    'generation_mw': None,
                    'production_type': None,
                    'resolution': resolution_text,
                    'quality_flag': 'actual',
                })
        
        return records
    
    def get_generation_data(
        self,
        country_code: str,
        start_date: str,
        end_date: str,
        production_type: Optional[str] = None,
        bidding_zone: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """
        Get actual generation data by production type.
        
        Args:
            country_code: ISO 2-letter country code
            start_date: Start date in format 'YYYY-MM-DD' or 'YYYY-MM-DDTHH:MM'
            end_date: End date in format 'YYYY-MM-DD' or 'YYYY-MM-DDTHH:MM'
            production_type: Optional production type code (if None, gets all types)
            bidding_zone: Optional bidding zone code
            
        Returns:
            List of generation data records
        """
        # Get bidding zone for country
        if bidding_zone is None:
            zones = COUNTRY_BIDDING_ZONES.get(country_code, [])
            if not zones:
                logger.warning(f"No bidding zones found for country {country_code}")
                return []
            bidding_zone = zones[0]
        
        # Build request parameters
        # ENTSO-E API requires YYYYMMDDHHMM format (12 digits)
        # Convert date strings to proper format
        def format_date_for_api(date_str: str, is_end: bool = False) -> str:
            """Format date string to ENTSO-E API format YYYYMMDDHHMM."""
            # Remove separators
            clean_date = date_str.replace('-', '').replace(':', '').replace('T', '').replace(' ', '')
            # If only date (8 chars), add time: 0000 for start, 2300 for end
            if len(clean_date) == 8:
                if is_end:
                    return clean_date + '2300'  # End of day
                else:
                    return clean_date + '0000'  # Start of day
            # If already has time, pad to 12 chars if needed
            elif len(clean_date) < 12:
                clean_date = clean_date.ljust(12, '0')
            return clean_date[:12]  # Ensure exactly 12 digits
        
        params = {
            'securityToken': self.api_token or '',
            'documentType': 'A75',  # Actual Generation per Production Type
            'processType': 'A16',   # Realised
            'outBiddingZone_Domain': bidding_zone,
            'periodStart': format_date_for_api(start_date, is_end=False),
            'periodEnd': format_date_for_api(end_date, is_end=True),
        }
        
        if production_type:
            params['psrType'] = production_type
        
        root = self._make_request(params)
        if root is None:
            return []
        
        # Parse generation data from XML
        records = []
        time_series = root.findall('.//{*}TimeSeries')
        
        for ts in time_series:
            # Get production type
            psr_type = ts.find('.//{*}psrType')
            prod_type = psr_type.text if psr_type is not None else None
            
            period = ts.find('.//{*}Period')
            if period is None:
                continue
            
            resolution = period.find('.//{*}resolution')
            resolution_text = resolution.text if resolution is not None else 'PT60M'
            
            points = period.findall('.//{*}Point')
            for point in points:
                position = point.find('.//{*}position')
                quantity = point.find('.//{*}quantity')
                
                if position is None or quantity is None:
                    continue
                
                # Calculate timestamp
                start_time = period.find('.//{*}start')
                if start_time is None:
                    continue
                
                start_dt = datetime.fromisoformat(start_time.text.replace('Z', '+00:00'))
                position_num = int(position.text) - 1
                
                if resolution_text == 'PT60M' or resolution_text == 'PT1H':
                    timestamp = start_dt + timedelta(hours=position_num)
                elif resolution_text == 'PT15M':
                    timestamp = start_dt + timedelta(minutes=15 * position_num)
                else:
                    timestamp = start_dt + timedelta(hours=position_num)
                
                records.append({
                    'timestamp_utc': timestamp.isoformat(),
                    'country_code': country_code,
                    'bidding_zone': bidding_zone,
                    'load_mw': None,
                    'generation_mw': float(quantity.text),
                    'production_type': prod_type,
                    'resolution': resolution_text,
                    'quality_flag': 'actual',
                })
        
        return records
    
    def get_combined_data(
        self,
        country_code: str,
        start_date: str,
        end_date: str,
        bidding_zone: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """
        Get both load and generation data for a country.
        
        Args:
            country_code: ISO 2-letter country code
            start_date: Start date in format 'YYYY-MM-DD' or 'YYYY-MM-DDTHH:MM'
            end_date: End date in format 'YYYY-MM-DD' or 'YYYY-MM-DDTHH:MM'
            bidding_zone: Optional bidding zone code
            
        Returns:
            Combined list of load and generation records
        """
        load_data = self.get_load_data(country_code, start_date, end_date, bidding_zone)
        generation_data = self.get_generation_data(country_code, start_date, end_date, None, bidding_zone)
        
        # Combine data (in a real implementation, you might want to merge by timestamp)
        return load_data + generation_data
