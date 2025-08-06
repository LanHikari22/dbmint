#!/bin/sh

# Needs to be hardcoded to run from web
app_name="dbmint" # variable created with edit_app_name.sh 

export TAG=latest

get_container_id() {
  docker ps -a --no-trunc --filter name=^/$app_name$ | sed -n 2p | cut -d' ' -f1
}

container_id=$(get_container_id)

if [ ! -z $container_id ]; then
  docker exec -it --user root $container_id sh
else
    docker run -d \
      -v $MOUNT:/mnt/ \
      --user root \
      --name $app_name \
      lan22h/$app_name:$TAG
    
    container_id=$(get_container_id)
    
    if [ ! -z $container_id ]; then
      docker exec -it $container_id sh
    fi
fi


# User quit the shell, kill the container

container_id=$(get_container_id)

if [ ! -z $container_id ]; then
  echo "Bye!"
  docker container stop $container_id -t 1
  docker rm $container_id
fi
