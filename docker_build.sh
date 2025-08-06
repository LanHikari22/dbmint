app_name="dbmint" # variable created with edit_app_name.sh 

docker build \
  -t lan22h/$app_name:latest \
  --build-arg USE_SHELL=$USE_SHELL \
  .
