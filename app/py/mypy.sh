#!/bin/bash

app_name="dbmint" # variable created with edit_app_name.sh 

script_dir=$(dirname "$(realpath "$0")")
project_dir="$script_dir/../.."

script -q -c "cd $project_dir/app/src; mypy --strict -m $app_name.main" /dev/null | tac
