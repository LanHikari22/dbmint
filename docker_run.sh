#!/bin/sh

# The port to expose SSH
#export SSH_PORT=2225

# Mount points for development.
export MOUNT_POINT=$HOME/data/mounted/$(cat app_name)/

docker run -it -d \
  -v $MOUNT_POINT:/mnt/ \
  --name $(cat app_name) \
  $(cat app_name)

#  -p $SSH_PORT:22 \
