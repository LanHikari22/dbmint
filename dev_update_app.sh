#!/bin/sh

# Refresh content of /app/
docker exec $(cat app_name) rm -rf /app/
docker exec $(cat app_name) mkdir -p /app/
docker cp app/. $(cat app_name):/app/
