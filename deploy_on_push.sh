#!/usr/bin/env bash
# ln -s "$(pwd)/deploy_on_push.sh" .git/hooks/pre-push


function build_zip() {
  mkdir "./target/zip"

  cargo build --release --target=x86_64-unknown-linux-musl

  cp "./target/x86_64-unknown-linux-musl/release/auxv-dot-org" "./target/zip/auxv-dot-org"
  cp -r "./pages" "./target/zip/pages"

  (cd "./target/zip" && zip -r "./auxv-dot-org.zip" "." -x "./auxv-dot-org.zip")
}

function deploy_zip() {
  local user="$1"
  local host="$2"
  local secret_path="$3"

  scp -i "$secret_path" "./target/zip/auxv-dot-org.zip" "$user@$host:~/auxv-dot-org.zip"

  # sudo to remove ./auxv-dot-org becaues lets_encrypt_cache is root
  ssh -i "$secret_path" "$user@$host" "mkdir ./auxv-dot-org-tmp \
  && unzip ./auxv-dot-org -d ./auxv-dot-org-tmp \
  && cp -r ./auxv-dot-org/lets_encrypt_cache ./auxv-dot-org-tmp/lets_encrypt_cache \
  && sudo -S rm -r ./auxv-dot-org \
  ; mv ./auxv-dot-org-tmp ./auxv-dot-org \
  && sudo -S systemctl restart auxv-dot-org \
  && rm ./auxv-dot-org.zip"

}


# Format: [user host secret_path]
declare -a deployments=(
  "ghostbird acceptance.auxv.org top_secret_do_not_share/acceptance.key"
  "root authority.auxv.org top_secret_do_not_share/authority.key"
)


exec < /dev/tty

branch=$(git rev-parse --abbrev-ref HEAD)
if [ "$branch" != "main" ]; then
  exit 0
elif ! git diff --quiet; then
    echo "Error: You have unstaged changes."
    echo "Please stage or stash your changes before deploying."
    exit 1
elif ! git diff --cached --quiet; then
    echo "Error: You have staged but uncommitted changes."
    echo "Please commit your changes before deploying."
    exit 1
fi

echo "Building zip:"
rm -r "./target/zip"
build_zip

for deployment in "${deployments[@]}"; do
  read -r user host secret <<< "$deployment"
  echo -e "\n\nDeploying to [$user@$host]:"

  deploy_zip "$user" "$host" "$secret"
  echo "-----------------------------------"
done

echo "Deployment completed"
