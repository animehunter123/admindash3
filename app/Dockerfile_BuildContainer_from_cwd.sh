#!/bin/bash

set -e # To ERROR OUT IF ANY COMMAND FAILS in the build process fails.

# This will build a admindash3:latest to appear in your `docker images`, note
# IT WILL NEED ./data.json (or respective deps) so make sure you are CD'd into
# the main ./app dir which has the ./data.json and/or similar deps.

# Grab the "version-githash" which will be used for the docker container version we build.
version=$(cat Cargo.toml | grep '^version =' | sed 's/version = "//' | sed 's/"$//')
version="${version}-$(git rev-parse --short HEAD)"

# Compile the latest dioxus version now (IF YOU WANT)
read -p "INSTALL: Do you need to install dioxus/rust on your debian host? (y/N): " user_choice
if [[ "$user_choice" =~ ^[Yy]$ ]]; then
  # 1. Update and install packages cleanly
  apt-get update && apt-get install -y curl git build-essential pkg-config libssl-dev
  # 2. Install Rustup safely
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # 3. Source the environment instead of hardcoding a broken tilde path
  . "$HOME/.cargo/env"
  # 4. Install cargo-binstall
  curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
  # 5. Install dioxus-cli into the user's actual cargo directory
  cargo binstall dioxus-cli -y --force
fi

echo "Cleaning ./target and to make admindash3:$version container compressed with release-only..."
rm -rf ./target/

echo "DOCKER: Stopping any existing containers just to ensure file handles are clear before compiling anything."
docker compose down

# Compiling the helper tool and copying it to the top level for migrating v1 buttons to v2 button json files.
echo "COMPILE: Rebuilding helper tool for json v1 to json v2 migration of buttons files."
cargo build --release --bin data_button_v2_migrator
cp ./target/release/data_button_v2_migrator .
cargo build --release --bin data_button_v2_to_v3_migrator
cp ./target/release/data_button_v2_to_v3_migrator .
rm -rf ./target/

# Compiling the dioxus webapp release version now FROM THE HOST LINUX HOST.
echo "COMPILE: Rebuilding the release binary with the EMBEDDED GIT COMMIT HASH.. into target/dx/Admindash3/release/"
rm -rf target/dx/Admindash3/release/ 2>/dev/null 1>/dev/null
dx bundle --web --release

docker build \
  -t admindash3:$version \
  -t admindash3:latest \
  .

read -p "EXPORT: Do you want to export the docker+sourcecode of admindash3:$version to /tmp? (y/N): " export_choice
if [[ "$export_choice" =~ ^[Yy]$ ]]; then
  cd .. # Because i moved code into ./app
  echo "Saving Docker Tag command..."
  rm -f /tmp/admindash3_loadnotesss_$version.txt
  echo "To Load: 1. If you load the docker tar, DONT FORGET TO  do: docker image tag admindash3:latest admindash3:$version" >>/tmp/admindash3_loadnotesss_$version.txt
  echo "To Load: 2. Also if migrating a older admindash you can do: cd /opt; mkdir admindash3 ; cd admindash3 ; tar xvf ../admindash3_sourcecodes_*.tar -C . ; " >>/tmp/admindash3_loadnotesss_$version.txt
  echo "To Load: 3. Ensure that you keep track of the live version of data_navbar.json and data_buttons.json and CHMOD 777 them." >>/tmp/admindash3_loadnotesss_$version.txt
  echo "To Load: 4. Finally confirm port 80 or 8080 in docker-compose.yml on destination + creds as needed." >>/tmp/admindash3_loadnotesss_$version.txt
  echo "To Load: 5. Dont forget to keep it running in cron or something like: crontab -e >> @reboot cd /opt/admindash3/app/ ; /usr/bin/docker compose down ; /usr/bin/docker compose up -d" >>/tmp/admindash3_loadnotesss_$version.txt
  echo "Saving Source code..."
  tar cvfpz /tmp/admindash3_sourcecodes_$version.tar --exclude="*target*" ./
  cd - # Because i moved code into ./app
  echo "Saving Docker image..."
  docker save admindash3:latest >/tmp/admindash3_dockerimage_$version.tar
  ls -l /tmp/admindash3_dockerimage_$version.tar
  echo "Compressing Docker image..."
  bzip2 /tmp/admindash3_dockerimage_$version.tar
  echo "Here is /tmp now of *admindash3*..."
  ls -lrtha /tmp/*admindash3*
  echo "Finished compressing."
  echo "1. If you load the docker tar, DONT FORGET TO: docker image tag admindash3:latest admindash3:$version"
  echo "2. Ensure that you keep track of the live version of data_navbar.json and data_buttons.json and CHMOD 777 them."
  echo "3. Finally confirm port 80/8080 and admin/admin creds in docker-compose.yml on destination as needed."
  echo "4. Dont forget to keep it running in cron or something like: crontab -e >> @reboot cd /opt/admindash3/app/ ; /usr/bin/docker compose down ; /usr/bin/docker compose up -d"
fi

echo "Script complete."
