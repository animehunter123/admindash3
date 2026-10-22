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

# RUSTENV: Clean the GLOBAL Cargo dependency caches so the exported environment
# contains mostly this project's dependencies instead of every old project's cache.
# WARNING: This affects OTHER Rust projects on this machine too.
echo "RUSTENV: Cleaning global Cargo dependency caches..."
echo "WARNING: This affects OTHER Rust projects on this machine too."
rm -rf "$HOME/.cargo/registry" "$HOME/.cargo/git"
# RUSTENV: Dioxus web builds need the wasm32-unknown-unknown Rust target.
echo "RUSTENV: Ensuring wasm32-unknown-unknown target is installed for Dioxus web builds..."
rustup target add wasm32-unknown-unknown
# RUSTENV: Fetch the exact dependencies in Cargo.lock for both the Linux host
# build and the Dioxus web/WASM build. This happens ONCE at the top.
echo "RUSTENV: Fetching locked dependencies for x86_64 Linux and wasm32 web builds..."
cargo fetch --locked --target x86_64-unknown-linux-gnu
cargo fetch --locked --target wasm32-unknown-unknown

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

cargo build --release --bin navbar_v1_to_v2_migrator 
cp ./target/release/navbar_v1_to_v2_migrator .

rm -rf ./target/

# Compiling the dioxus webapp release version now FROM THE HOST LINUX HOST.
echo "COMPILE: Rebuilding the release binary with the EMBEDDED GIT COMMIT HASH.. into target/dx/Admindash3/release/"
rm -rf target/dx/Admindash3/release/ 2>/dev/null 1>/dev/null
dx bundle --web --release

# VERIFY: Confirm that both debug and release builds work with no network access,
# using the Cargo cache that will be included in the Rust environment archive.
echo "VERIFY: Testing fully offline debug and release builds..."
cargo build --locked --offline
cargo build --locked --offline --release

# RUSTENV: Export the Rust toolchain and this project's Cargo dependency cache.
# This is an UNCOMPRESSED tar because compression was slow; restore it into $HOME.
read -p "EXPORT: Do you want to export the rust compile environment of admindash3:$version to /tmp? (y/N): " rustenv_choice
if [[ "$rustenv_choice" =~ ^[Yy]$ ]]; then
  echo "EXPORT: Creating Rust environment archive..."
  rm -f "/tmp/admindash3_rustenviron_${version}.tar"
  tar --create \
    --file "/tmp/admindash3_rustenviron_${version}.tar" \
    --directory "$HOME" \
    .cargo .rustup
  ls -lh "/tmp/admindash3_rustenviron_${version}.tar"
fi

# Remove other compiled targets and only leave the webserver binary - to make the container decrease from 500mb to only 40mb!!!
rm -rf /tmp/web
mv ./target/dx/Admindash3/release/web/ /tmp
rm -rf ./target/
mkdir -p ./target/dx/Admindash3/release/
mv /tmp/web ./target/dx/Admindash3/release/

# Use docker build instead of buildx until Dioxus docs say to do so in the future.
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
  echo "To Restore Rust environment: 6. From your HOME directory, run: tar xf /path/to/admindash3_rustenviron_$version.tar -C \$HOME ; then add ~/.cargo/bin to PATH." >>/tmp/admindash3_loadnotesss_$version.txt
  echo "To Restore Rust environment: 7. For bash, run: source \$HOME/.cargo/env ; for fish, run: source \$HOME/.cargo/env.fish ; or permanently run: fish_add_path \$HOME/.cargo/bin" >>/tmp/admindash3_loadnotesss_$version.txt
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
  echo "5. To restore the Rust environment, from your HOME directory run: tar xf /path/to/admindash3_rustenviron_$version.tar -C \$HOME"
  echo "6. This restores ~/.cargo and ~/.rustup. For bash, run: source \$HOME/.cargo/env ; for fish, run: source \$HOME/.cargo/env.fish"
  echo "7. To permanently add cargo/rustc to fish PATH, run: fish_add_path \$HOME/.cargo/bin ; then verify with: cargo --version ; rustc --version ; dx --version"
fi

echo "Script complete."
