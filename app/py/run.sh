#!/bin/bash

app_name="dbmint" # variable created with edit_app_name.sh 

script_dir=$(dirname "$(realpath "$0")")
project_dir="$script_dir/../.."

PYTHONPATH=$project_dir/app/py/src python3 -m $app_name.main $@
