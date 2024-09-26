if [ "$(cat is_persistent)" = "Yes" ]; then
  docker kill $(cat app_name) || echo '' > /dev/null
  docker rm $(cat app_name)
fi

docker rmi lan22h/$(cat app_name):latest || \
echo 'Nothing to clean!'
