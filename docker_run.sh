#!/bin/sh

# The port to expose SSH
export SSH_PORT=2225

# Mount points for development.
export MOUNT_POINT=$HOME/data/mounted/dbmint/

docker run -it -d \
  -p $PUB_SSH_PORT:22 \
  -v $MOUNT_POINT:/mnt/ \
  --name dbmint \
  dbmint
