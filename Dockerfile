# Use latest Ubuntu version as base
FROM alpine:latest

ARG USE_SHELL=0

# Install necessary packages
RUN apk update && apk add --no-cache \
  python3 \
  py3-pip \
  gcc \
  git \
  sqlite

#ARG USE_SHELL 
#RUN echo "export USE_SHELL=$USE_SHELL" > /usr

# Create a Python virtual environment
RUN python3 -m venv /opt/venv

COPY app/py /app/
COPY app/rust/target/x86_64-unknown-linux-musl/release/dbmint-rs /usr/local/bin/dbmint-rs
RUN chmod +x /usr/local/bin/dbmint-rs

# Let's install the requirements by the app
RUN . /opt/venv/bin/activate && pip install -r /app/requirements.txt

ARG USE_SHELL
RUN /bin/sh -c '\
  if [ $USE_SHELL ]; then \
    echo "yes" > /var/use_shell; \
  else \
    echo "no" > /var/use_shell; \
  fi \
'

# Copy the entrypoint script
COPY boot/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Use the custom entrypoint
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
