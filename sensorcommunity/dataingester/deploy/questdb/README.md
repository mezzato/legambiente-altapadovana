## Set up user and permissions

```bash
sudo useradd -m questdb
sudo usermod -a -G sudo questdb
sudo mkdir -p /var/lib/questdb/
sudo chown -R questdb:questdb /var/lib/questdb/
sudo chmod 755 /var/lib/questdb/
```

## Install and deploy executable

see [installation steps](https://questdb.com/docs/quick-start/#install-questdb)

## Deploy systemd unit

```bash
sudo cp questdb.service /etc/systemd/system
sudo systemctl enable questdb.service
sudo systemctl daemon-reload
echo start the service
sudo systemctl start questdb.service
```

# Set up the operating system

see [OS configuration](https://questdb.com/docs/operations/capacity-planning/#os-configuration)

# Set up partitioning

See [Database partitioning](https://questdb.com/glossary/database-partitioning/)
