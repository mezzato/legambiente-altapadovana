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

mirror_path = "./mirror_last5mins"
mpath = basepath.joinpath(mirror_path)
conn_file = basepath.joinpath("./config.json")

if not conn_file.exists():
    print(f'config file not found {conn_file}')
    exit()

try:
    mpath.mkdir(parents=True, exist_ok=True)
except OSError as exc:
    if exc.errno != errno.EEXIST:
        raise
    pass

user_agents = [
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36'
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36'
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36'
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36'
    'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36'
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.1 Safari/605.1.15'
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 13_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.1 Safari/605.1.15'
]

print(f"importing data")
sensors_file_path = basepath.joinpath('./sensors.csv')
chips_file_path = basepath.joinpath('./chips.csv')

chip_info_by_id = {}

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
            description=row[4]
            chip_info_by_id[chip_id] = (lat,lon,city,description)

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
            sensor_type=row[2].upper

            chip_info = chip_info_by_id.get(chip_id)
            if chip_info is None:
                continue

            city = chip_info[2]
            info = chip_info[3]

            filename= mpath.joinpath(sensor_id + ".json")
            
            # Download the file from `url` and save it locally under `file_name`

            print(f"importing sensor id {sensor_id}")
            url = f'https://data.sensor.community/airrohr/v1/sensor/{sensor_id}/'
            # print(f"Downloading {url}\n")
            try:
                headers = {
                    'User-Agent': random.choice(user_agents),
                    'Origin': 'http://example.com',
                    'Referer': 'http://example.com/some_page', 
                }
                response = requests.get(url, headers=headers, verify=False, timeout=10)
                response.raise_for_status()
            except requests.exceptions.HTTPError as e:
                # Return code error (e.g. 404, 501, ...)
                # ...
                print('HTTPError: {}, URL: {}, status code: {}'.format(e.args[0], url, response.status_code))
                continue
            except requests.exceptions.ConnectionError as e:
                # Not an HTTP-specific error (e.g. connection refused)
                # ...
                print('ConnectionError: {}, URL: {}'.format(e, url))
                continue
            except requests.exceptions.Timeout as e:
                print('Timeout error: {}, URL: {}'.format(e, url))
                continue
            except requests.exceptions.RequestException as e:
                print('Request error: {}, URL: {}'.format(e, url))
                continue
            else:
                # 200
                # ...
                # with open(filename, 'wb') as out_file:
                #    shutil.copyfileobj(response.text, out_file)

                print('response OK: {}, status code: {}, downloaded: {} bytes, from: {}'.format(response.ok, response.status_code, len(response.content), url))
                with open(filename, 'w') as f:
                    f.write(response.text)
                
            
            with open(filename, "r") as json_file:
                data = json.load(json_file)

                # Iterate through the JSON array 
                for item in data:
                    point = Point("particulate")
                    for sensorvalue in item["sensordatavalues"]:
                        point.field(sensorvalue["value_type"], float(sensorvalue["value"]))
                    
                    # sensor_id;sensor_type;location;lat;lon;timestamp;P1;durP1;ratioP1;P2;durP2;ratioP2
                    # 62574;SDS011;77629;45.630;11.704;2024-10-30T00:01:08;15.30;;;8.98;;
                    # "#datatype tag,tag,tag,double,double,dateTime:2006-01-02T15:04:05,
                    # double,double,double,double,double,double"
                    
                    point.tag("chip_id", chip_id)
                    point.tag("city", city)
                    point.tag("info", info)
                    point.tag("sensor_id", item["sensor"]["id"])
                    point.tag("sensor_type", item["sensor"]["sensor_type"]["name"])
                    point.tag("location", item["location"]["id"])
                    point.tag("lat", float(item["location"]["latitude"]))
                    point.tag("lon", float(item["location"]["longitude"]))
                    point.time(parser.parse(item["timestamp"]))
                    # print(point.to_line_protocol())
                    points.append(point)

                    
            # remove the file
            os.remove(filename)

        if len(points) > 0:
            with InfluxDBClient.from_config_file(conn_file) as client:
                with client.write_api(write_options=SYNCHRONOUS) as writer:
                    try:
                        writer.write(bucket="sensorcommunity", record=points)
                    except InfluxDBError as e:
                        print(f'InfluxDB error: {e}')
        else:
            print(f'No data to import into InfluxDB')

mpath.rmdir()
