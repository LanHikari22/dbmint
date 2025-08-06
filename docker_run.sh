#!/bin/sh

# The port to expose SSH
#export SSH_PORT=2225

app_name="dbmint" # variable created with edit_app_name.sh 

# Mount points for development.
if [ -z $MOUNT ]; then
  export MOUNT=$HOME/data/mounted/$app_name/
fi

if [ "$(cat is_persistent)" = "Yes" ]; then
  docker run -it -d \
    -v $MOUNT:/mnt/ \
    --user $(id -u):$(id -g) \
    --name $app_name \
    lan22h/$app_name:latest
else
  # Action on "No"
  docker run --rm -it \
    -v $MOUNT:/mnt/ \
    --user $(id -u):$(id -g) \
    --name $app_name \
    lan22h/$app_name:latest \
    $@
fi
#  -p $SSH_PORT:22 \
