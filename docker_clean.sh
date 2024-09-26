docker kill $(cat app_name) || echo '' > /dev/null
docker rm $(cat app_name) && \
docker rmi $(cat app_name) || \
echo 'Nothing to clean!'
