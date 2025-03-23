#! /bin/bash

echo -n "Install InfluxDB3 Python client"

sudo apt install python3-pip

# use python3 --version to determine the specific version
sudo apt install python3.12-venv

python3 -m venv .venv
.venv/bin/pip install influxdb3-python python-dateutil requests

