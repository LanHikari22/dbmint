#!/bin/bash

script_dir=$(dirname "$(realpath "$0")")
project_dir="$script_dir/../.."

python3 -m pip install --upgrade -r $project_dir/app/py/requirements.txt
