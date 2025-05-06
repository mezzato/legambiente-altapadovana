#!/bin/bash
set -eo pipefail

EXE="$1"
[[ -z "$EXE" ]] && { echo "error: executable name not specified"; exit 1; }


echo "deploying $EXE as dataingester service"

serviceCommand() {
  if sudo systemctl is-active --quiet ${1}; then
    sudo service ${1} ${2}
  fi
}


SYSDDIR=/etc/systemd/system

file="./$EXE"
if [ -f "$file" ]
then
	echo "$file found."
else
	echo "$file not found. You need to build the executable first."
  exit 1
fi

if [ -d "$SYSDDIR" ]
then
	echo "$SYSDDIR found."
else
	echo "$SYSDDIR not found. Your system needs to support systemd."
    exit 2
fi

EXEC=/usr/local/bin/dataingester
PIDFILE=/var/run/dataingester.pid
CONFDIR="/etc/dataingester"
CONFFILE="dataingester.toml"
CONF="$CONFDIR/$CONFFILE"
SERVICENAME=dataingesterd
LOGDIR="/var/log/dataingester"
USER=dataingester

echo stop the service
serviceCommand $SERVICENAME stop

sudo mkdir -p $LOGDIR
sudo chown $USER:$USER -R $LOGDIR
sudo chmod 755 $LOGDIR

if [ -f $EXEC ]; then
  echo move old binary to $EXEC.old
  sudo mv $EXEC $EXEC.old
fi
echo copy binary to $EXEC
sudo cp $file $EXEC

if [ -f $CONF ]; then
  echo move old conf to $CONF.old
  sudo mv $CONF $CONF.old
fi

echo copy new conf
sudo mkdir -p $CONFDIR
sudo cp $CONFFILE $CONF

if [ -f $SYSDDIR/$SERVICENAME.service ]; then
  echo move old systemd configuration to $SYSDDIR/$SERVICENAME.service.old
  sudo mv $SYSDDIR/$SERVICENAME.service $SYSDDIR/$SERVICENAME.service.old
fi
echo copy systemd configuration
sudo cp $SERVICENAME.service $SYSDDIR/$SERVICENAME.service
# sudo cp forking-$SERVICENAME.service $SYSDDIR

echo enable service at boot
sudo systemctl enable $SERVICENAME.service

# echo Redis requires
# https://www.digitalocean.com/community/tutorials/how-to-configure-a-linux-service-to-start-automatically-after-a-crash-or-reboot-part-2-reference
# https://superuser.com/questions/1225079/referencing-sysv-init-scripts-as-systemd-unit-file-dependency
# sudo ln -s /etc/init.d/redis $SYSDDIR/domuskern.target.requires/redis

echo reload systemctl daemon
sudo systemctl daemon-reload

echo start the service
sudo systemctl start $SERVICENAME.service
