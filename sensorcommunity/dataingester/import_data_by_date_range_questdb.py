#!.venv/bin/python3

import os
from dateutil import parser
from questdb.ingress import Sender, IngressError, TimestampNanos

import requests
import urllib3
import csv
import random

from pathlib import Path
import errno

import os
basepath = Path(os.path.dirname(os.path.abspath(__file__)))

import sensor_cache;

mirror_path = "./mirror"
mpath = basepath.joinpath(mirror_path)

import os
import sys
from datetime import datetime, timedelta

def get_environment_variables():
    """
    Get environment variables START, END, INCLUDE_REGISTERED_SENSORS and SENSORS.
    Set default values if they are not set.
    
    Returns:
        tuple: (start_datetime, end_datetime, sensors_list, include_registered_sensors, config)
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

    # Get SENSORS environment variable with default
    include_registered_sensors_str = os.environ.get("INCLUDE_REGISTERED_SENSORS")
    if include_registered_sensors_str:
        # convert to bool
        include_registered_sensors = include_registered_sensors_str.strip().lower() == "true"
    else:
        # Default sensors list
        include_registered_sensors = False
        print(f"INCLUDE_REGISTERED_SENSORS environment variable not set. Using default: {include_registered_sensors}")
    
    """
    Retrieves QuestDB configuration from environment variables.
    
    Returns:
        Dict containing the QuestDB configuration parameters.
        
    Raises:
        SystemExit: If any required environment variables are missing.
    """
    
    required_vars = [
        "QDB_CLIENT_CONF",
        "QDB_TABLE",
    ]

    optional_vars =  [
        "QDB_CLIENT_ORG"
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
    

    return start_datetime, end_datetime, sensors_list, include_registered_sensors, config

def safe_float_convert(value):
    try:
        return float(value)
    except ValueError:
        return None
    except TypeError:
        return None

def main():

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

    # Get environment variables
    start, end, sensor_ids, include_registered_sensors, questdb_config = get_environment_variables()
    
    # Print the parsed variables
    print("\nParsed Environment Variables:")
    print(f"START: {start}")
    print(f"END: {end}")
    print(f"SENSORS: {sensor_ids}")
    print(f"INCLUDE REGISTERED SENSORS: {include_registered_sensors}")
    
    # Your program logic would go here
    print("\nTime range duration:", end - start)
    print(f"Number of sensors: {len(sensor_ids)}")
    
    # Example processing
    print("\nProcessing data for sensors:")


    current_date = start
    urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

    while current_date <= end:
        for sensor in sensor_cache.sensors:

            chip_id = sensor['chip_id']
            if not include_registered_sensors and not chip_id.startswith("_"):
                print('Skipping chip id: {}'.format(chip_id))
                continue
            sensor_id = sensor['sensor_id']

            if len(sensor_ids) > 0 and not sensor_id in sensor_ids:
                continue

            download_date=current_date.strftime("%Y-%m-%d")
            sensor_type = sensor['sensor_type']

            file=f'{download_date}_{sensor_type.lower()}_sensor_{sensor_id}.csv'
            url = f'https://archive.sensor.community/{download_date}/{file}'


            filename= mpath.joinpath(file)
            
            # Download the file from `url` and save it locally under `file_name`

            print(f"importing sensor id {sensor_id} from {url}")
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
                
            
            with open(filename, "r") as csv_file:

                print(f"parsing {filename}")
                csv_reader = csv.reader(csv_file, delimiter=';')
                headers = next(csv_reader, None)

                # Use the with...as statement to ensure the file is properly closed and resources
                # are released.
                with Sender.from_conf(questdb_config.get("QDB_CLIENT_CONF"), tls_verify=False) as sender:
                    # Record with provided designated timestamp (using the 'at' param)
                    # Notice the designated timestamp is expected in Nanoseconds,
                    # but timestamps in other columns are expected in Microseconds.
                    # You can use the TimestampNanos or TimestampMicros classes,
                    # or you can just pass a datetime object

                    for row in csv_reader:
                        if len(row) == 0:
                            continue

                        table = questdb_config.get("QDB_TABLE")

                        chip_id = sensor['chip_id']
                        city = sensor['city']
                        info = sensor['info']
                        sensor_id = sensor['sensor_id']
                        sensor_type = sensor['sensor_type']
                        # location = row[2]
                        lat = row[3] # safe_float_convert(row[3])
                        lon = row[4] # safe_float_convert(row[4])
                        time = parser.parse(row[5])

                        for i in range(6, len(headers)):
                            val = safe_float_convert(row[i])
                            if val is not None:
                                 sender.row(
                                    table,
                                    symbols={
                                        "chip_id": chip_id,
                                        "city" : city,
                                        "info" : info,
                                        "sensor_id": sensor_id,
                                        "sensor_type": sensor_type,
                                        # "location" : location,
                                        "lat" : lat,
                                        "lon" : lon,
                                        "field": headers[i],
                                    },
                                    columns={
                                        "value": val,
                                    },
                                    at=time
                                )
                    
                    # You can flush manually at any point.
                    sender.flush()

            # remove the file
            os.remove(filename)
        
        current_date += timedelta(days=1)
    
    mpath.rmdir()

if __name__ == "__main__":
    main()
