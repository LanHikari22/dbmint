#!/bin/sh

# Modify the prompt to show this is a dbmint system
echo "export PS1=\"dbmint:\W# \"" >> /etc/profile

echo "Welcome to the dbmint terminal!" > /etc/motd
echo >> /etc/motd
echo 'Users are not adviced to use this.' >> /etc/motd
echo 'dbmint is recommended to be used as an external command.' >> /etc/motd
echo >> /etc/motd
echo 'You may change this message by editing /etc/motd.' >> /etc/motd

ssh-keygen -A

/usr/sbin/sshd -D

# Stay on indefinitely
tail -f /dev/null
