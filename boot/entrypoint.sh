#!/bin/sh

if [ $(cat /var/use_shell) == "yes" ]; then
  # Stay indefinitely
  tail -f /dev/null
else
  # Just run the app
  . /opt/venv/bin/activate && \
    PYTHONPATH=/app/src python3 -m dbmint.main $@
fi
