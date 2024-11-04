# Useful snippets

## How to give permission to a bucket

influx v1 auth create --username 'enrico' --write-bucket 'e3df6bfa8a2fbd05' --password 'whatever' --org 'legambiente' --skip-verify

## How to delete a measure

```bash
influx delete --bucket sensorcommunity --predicate '_measurement="particulate"' --start '2009-01-02T23:00:00Z' --stop '2025-01-02T23:00:00Z'  --skip-verify
```

## How to install and run the python script

- Install the python 3 virtual environment:

  ```
  python3 -m venv .venv
  .venv/bin/pip install influxdb-client
  .venv/bin/pip install python-dateutil
  ```

- Generate the config.json file. 
  Use `influx config --json` to get the settings. Rember to use `"verify_ssl": false` for self-signed certificates.
