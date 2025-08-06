#!/bin/sh

script_dir=$(dirname "$(readlink -f "$0")")
only_dir=$(basename $script_dir)

files_to_process=$(find $script_dir -type f)

echo $files_to_process | tr ' ' '\n' | while read file; do 
  if [ "$file" = "$script_dir/edit_app_name.sh" ] || \
     [ "$file" = "$script_dir/.edit_app_name.sh.swp" ]; then
    continue
  fi

  cat $file | grep -q "app_name="
  found=$?

  if [ $found -eq 0 ]; then
    echo OK $file

    sed -i "s/app_name=.*/app_name=\"$only_dir\" # variable created with edit_app_name.sh /" $file
  fi
done
