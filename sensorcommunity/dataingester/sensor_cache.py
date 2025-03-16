#!.venv/bin/python3

import os
from dateutil import parser
import json

from influxdb_client import InfluxDBClient, Point
from influxdb_client.client.write_api import SYNCHRONOUS
from influxdb_client.client.exceptions import InfluxDBError

import requests
import urllib3
import csv
import random

from pathlib import Path
import errno

import os
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