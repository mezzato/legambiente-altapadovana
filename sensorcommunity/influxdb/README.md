# Useful snippets

## How to give permission to a bucket

influx v1 auth create --username 'enrico' --write-bucket 'e3df6bfa8a2fbd05' --password 'whatever' --org 'legambiente' --skip-verify

## How to delete a measure

```bash
influx delete --bucket sensorcommunity --predicate '_measurement="particulate"' --start '2009-01-02T23:00:00Z' --stop '2025-01-02T23:00:00Z'  --skip-verify
```
