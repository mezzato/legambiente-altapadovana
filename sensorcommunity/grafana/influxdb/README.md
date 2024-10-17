# Useful snippets

## How to delete a measure

```bash
influx delete --bucket sensorcommunity --predicate '_measurement="particulate"' --start '2009-01-02T23:00:00Z' --stop '2025-01-02T23:00:00Z'  --skip-verify
```


