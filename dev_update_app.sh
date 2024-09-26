#!/bin/sh

# Refresh content of /app/
docker exec dbmint rm -rf /app/
docker exec dbmint mkdir -p /app/
docker cp app/. dbmint:/app/
