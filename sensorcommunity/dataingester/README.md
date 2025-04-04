
## How to install and run the python script

- Install the python 3 virtual environment:

  ```
  sudo apt install python3-pip

  # use python3 --version to determine the specific version
  sudo apt install python3.12-venv
  
  python3 -m venv .venv
  .venv/bin/pip install influxdb3-python python-dateutil requests
  ```

- Generate the config.json file. 
  Use `influx config --json` to get the settings. Rember to use `"verify_ssl": false` for self-signed certificates.

- Set up the cron job:

  ```bash
  crontab -e
  ```

  Add

  ```
  # At every 5th minute
  */5 * * * * /usr/bin/env bash -c 'cd /root/workspace/legambiente-altapadovana/sensorcommunity/influxdb/ && source .venv/bin/activate && ./import_data_last_5_min.py' > /dev/null 2>&1
  ```


  ## How to delete a measure

```bash
influx delete --bucket sensorcommunity --predicate '_measurement="particulate"' --start '2025-03-12T00:00:00Z' --stop '2025-03-16T23:00:00Z'  --skip-verify
```
