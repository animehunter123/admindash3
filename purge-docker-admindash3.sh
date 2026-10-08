#!/bin/bash

echo "ERASING ALL DOCKER IMAGES named 'admindash'... in your docker images list."

docker rm -f admindash3

docker rmi $(docker images 'admindash3')

echo "Script complete."

