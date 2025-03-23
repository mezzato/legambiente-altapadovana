#!.venv/bin/python3

import os
from dateutil import parser
import urllib3
import csv

import sys
from datetime import datetime, timedelta

from pathlib import Path

basepath = Path(os.path.dirname(os.path.abspath(__file__)))

sensors_file_path = basepath.joinpath('./sensors.csv')
chips_file_path = basepath.joinpath('./chips.csv')

chip_info_by_id = {}

sensors = []

try:
    csv_file = open(chips_file_path, 'r')
except FileNotFoundError:
    print(f'file not found {chips_file_path}')
else:
    with csv_file:
        print(f"parsing {chips_file_path} and loading cache")
        csv_reader = csv.reader(csv_file, delimiter=',')
        line_count = -1
        points = []

        for row in csv_reader:
            line_count += 1
            if line_count == 0:
                continue
            if len(row) == 0:
                continue
            chip_id=row[0]
            lat=row[1]
            lon=row[2]
            city=row[3]
            info=row[4]
            chip_info_by_id[chip_id] = (lat,lon,city,info)

try:
    csv_file = open(sensors_file_path, 'r')
except FileNotFoundError:
    print(f'file not found {sensors_file_path}')
else:
    with csv_file:
        print(f"parsing {sensors_file_path}")
        csv_reader = csv.reader(csv_file, delimiter=',')
        line_count = -1
        points = []

        urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

        for row in csv_reader:
            line_count += 1
            if line_count == 0:
                continue
            if len(row) == 0:
                continue
            chip_id=row[0]
            sensor_id=row[1]
            sensor_type=row[2].upper()

            chip_data = chip_info_by_id.get(chip_id)
            if chip_data is None:
                continue

            lat = chip_data[0]
            lat = chip_data[1]
            city = chip_data[2]
            info = chip_data[3]

            sensors.append({
                'chip_id': chip_id,
                'lat': lat,
                'lon': lon,
                'sensor_id': sensor_id, 
                'sensor_type': sensor_type,
                'city': city,
                'info': info,
            })


def get_environment_variables():
    """
    Get environment variables START, END, and SENSORS.
    Set default values if they are not set.
    
    Returns:
        tuple: (start_datetime, end_datetime, sensors_list, config)
    """
    # Get START environment variable with default
    start_str = os.environ.get("START")
    if start_str:
        try:
            # Parse using YYYY-MM-DD format
            start_datetime = datetime.strptime(start_str, "%Y-%m-%d")
        except ValueError:
            print(f"Error: START environment variable '{start_str}' is not in YYYY-MM-DD format.")
            sys.exit(1)
    else:
        # Default to yesterday
        yesterday = datetime.now() - timedelta(days=1)
        start_datetime = yesterday.replace(hour=0, minute=0, second=0, microsecond=0)
        print(f"START environment variable not set. Using default (yesterday): {start_datetime.strftime('%Y-%m-%d')}")
      
    # Get END environment variable with default
    end_str = os.environ.get("END")
    if end_str:
        try:
            # Parse using YYYY-MM-DD format
            end_datetime = datetime.strptime(end_str, "%Y-%m-%d")
        except ValueError:
            print(f"Error: END environment variable '{end_str}' is not in YYYY-MM-DD format.")
            sys.exit(1)
    else:
        # Default to yesterday
        yesterday = datetime.now() - timedelta(days=1)
        end_datetime = yesterday.replace(hour=0, minute=0, second=0, microsecond=0)
        print(f"END environment variable not set. Using default (yesterday): {end_datetime.strftime('%Y-%m-%d')}")
    
    # Validate that start is before end
    if start_datetime > end_datetime:
        print("Error: START date must be before END date.")
        sys.exit(1)
    
    # Get SENSORS environment variable with default
    sensors_str = os.environ.get("SENSORS")
    if sensors_str:
        # Split by comma and strip whitespace
        sensors_list = [sensor.strip() for sensor in sensors_str.split(",")]
    else:
        # Default sensors list
        sensors_list = []
        print(f"SENSORS environment variable not set. Using default: {', '.join(sensors_list)}")
    
    """
    Retrieves InfluxDB 3 configuration from environment variables.
    
    Returns:
        Dict containing the InfluxDB 3 configuration parameters.
        
    Raises:
        SystemExit: If any required environment variables are missing.
    """
    
    required_vars = [
        "INFLUXDB3_HOST",
        "INFLUXDB3_DATABASE",
        "INFLUXDB3_TOKEN",
        "INFLUXDB3_TABLE",
    ]

    optional_vars =  [
        "INFLUXDB3_ORG"
    ]
    
    config = {}
    missing_vars = []
    
    for var in required_vars:
        value = os.environ.get(var)
        if value is None:
            missing_vars.append(var)
        else:
            config[var] = value

    for var in optional_vars:
        value = os.environ.get(var)
        if value is not None:
            config[var] = value
    
    if missing_vars:
        print(f"Error: Missing required environment variables: {', '.join(missing_vars)}")
        sys.exit(1)
    

    return start_datetime, end_datetime, sensors_list, config