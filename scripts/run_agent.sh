
arch="0"
arch="$arch 3 0"
arch="$arch 0 1 0"

python -m autofl.agent \
  --arch="${arch}" \
  "$@"
