# Let's Encrypt

Follow the steps at: https://grafana.com/docs/grafana/latest/setup-grafana/set-up-https/

## How to restart Grafana automatically on renewal

Use a renew hook, add to `/etc/letsencrypt/renewal/yourdomain.conf`

```
renew_hook = systemctl restart grafana-server.service
```

or use the hook directory:

- create the script `/etc/letsencrypt/renewal-hooks/post/restart_grafana.sh`
- make sure it is executable: `sudo chmod a+x /etc/letsencrypt/renewal-hooks/post/restart_grafana.sh`
- The script might contain:

  ```bash
  #!/bin/bash
  systemctl restart grafana-server.service
  ```

The it with:
- `sudo certbot renew --dry-run --run-deploy-hooks`
- check the log: `sudo cat /var/log/letsencrypt/letsencrypt.log`

