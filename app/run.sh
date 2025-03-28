#!/bin/bash

script_dir=$(dirname "$(realpath "$0")")
project_dir="$script_dir/.."

PYTHONPATH=$project_dir/app/src python3 -m $(cat $project_dir/app_name).main $@
