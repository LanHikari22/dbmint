# Use latest Ubuntu version as base
FROM alpine:latest

# Install necessary packages
 RUN apk update && apk add --no-cache \
  python3 \
  py3-pip \
  gcc \
  sqlite

# Create a Python virtual environment
RUN python3 -m venv /opt/venv

# Activate the virtual environment and install the necessary Python package
# RUN . /opt/venv/bin/activate && pip install \
# dbml-sqlite

# Set up SSH
# RUN mkdir /var/run/sshd
# RUN echo 'root:root' | chpasswd
# RUN sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/' /etc/ssh/sshd_config
# 
# ENV NOTVISIBLE "in users profile"
# RUN echo "export VISIBLE=now" >> /etc/profile

# Set the timezone
# ENV TZ=America/Chicago
# RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

# Setup place for bins in $PATH
RUN mkdir -p /root/.local/bin
RUN echo 'PATH=/root/.local/bin:$PATH' >> /root/.zshrc

COPY app /app/

# Let's install the requirements by the app
RUN . /opt/venv/bin/activate && pip install -r /app/requirements.txt

# Expose ports
# EXPOSE 22

# Copy the entrypoint script
COPY scripts/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh
 
# Use the custom entrypoint
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
