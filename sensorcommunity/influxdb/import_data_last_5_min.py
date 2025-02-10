#!.venv/bin/python3

import os
from dateutil import parser
import json

from influxdb_client import InfluxDBClient, Point
from influxdb_client.client.write_api import SYNCHRONOUS
from influxdb_client.client.exceptions import InfluxDBError

import urllib.request, csv, shutil

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

print(f"importing data")
csv_file_name = basepath.joinpath('./sensors_by_city.csv')                 

try:
    csv_file = open(csv_file_name, 'r')
except FileNotFoundError:
    print(f'file not found {csv_file_name}')
else:
    with csv_file:
        print(f"parsing {csv_file_name}")
        csv_reader = csv.reader(csv_file, delimiter=',')
        line_count = -1
        points = []

        for row in csv_reader:
            line_count += 1
            if line_count == 0:
                continue
            sensor_id=row[0]
            sensor_type=row[1].upper
            chip_id=row[2]
            city=row[3]

            filename= mpath.joinpath(sensor_id + ".json")

            
            # Download the file from `url` and save it locally under `file_name`

            print(f"importing sensor id {}".format(sensor_id))
            url = f'https://data.sensor.community/airrohr/v1/sensor/{sensor_id}/'
            # print(f"Downloading {url}\n")
            try:
                response = urllib.request.urlopen(url)
            except urllib.error.HTTPError as e:
                # Return code error (e.g. 404, 501, ...)
                # ...
                print('HTTPError: {}, URL: {url}'.format(e.code))
                continue
            except urllib.error.URLError as e:
                # Not an HTTP-specific error (e.g. connection refused)
                # ...
                print('URLError: {}, URL: {url}'.format(e.reason))
                continue
            else:
                # 200
                # ...
                with open(filename, 'wb') as out_file:
                    shutil.copyfileobj(response, out_file)
                
            
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
                    point.tag("sensor_id", item["sensor"]["id"])
                    point.tag("sensor_type", item["sensor"]["sensor_type"]["name"])
                    point.tag("location", item["location"]["id"])
                    point.field("lat", float(item["location"]["latitude"]))
                    point.field("lon", float(item["location"]["longitude"]))
                    point.time(parser.parse(item["timestamp"]))
                    # print(point.to_line_protocol())
                    points.append(point)

                    
            # remove the file
            os.remove(filename)

        with InfluxDBClient.from_config_file(conn_file) as client:
            with client.write_api(write_options=SYNCHRONOUS) as writer:
                try:
                    writer.write(bucket="sensorcommunity", record=points)
                except InfluxDBError as e:
                    print(f'InfluxDB error: {e}')

mpath.rmdir()
