#!/bin/bash

script_dir=$(dirname "$(realpath "$0")")
project_dir="$script_dir/.."

script -q -c "cd $project_dir/app/src; mypy --strict -m $(cat $project_dir/app_name).main" /dev/null | tac
