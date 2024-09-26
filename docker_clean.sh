docker kill dbmint || echo '' > /dev/null
docker rm dbmint && \
docker rmi dbmint || \
echo 'Nothing to clean!'
