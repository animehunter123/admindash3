#!/bin/bash

echo Restarting admindash3 container...
cd /opt/admindash3/app/
/usr/bin/docker compose down
/usr/bin/docker compose up -d
