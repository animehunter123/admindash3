# This dockerfile requires that your compilation environment already have 
# git, dioxus, and the sourcecode installed and ready. It also requires that 
# you have built the latest dioxus and wasm builds via running:
# dx bundle --web --release 
#
# This dockerfile no longer uses the Dioxus docs official procedures b/c the git hash is embedded
# in the container, was: https://dioxuslabs.com/learn/0.7/tutorial/deploy



FROM ubuntu

# 1. Install system dependencies, git, and libraries needed to build Rust crates
RUN apt-get update && apt-get install -y \
    curl \
    git \
    vim \
    less \
    && rm -rf /var/lib/apt/lists/*

# Removed these
#    build-essential \
#    pkg-config \
#    libssl-dev \

# NOT NEEDED: 2. Install Rust & Cargo using rustup (safer and newer than apt-get cargo)
#RUN curl --proto '=https' --tlsv1.2 -sSf https://rustup.rs | sh -s -- -y
#ENV PATH="/root/.cargo/bin:${PATH}"

# NOT NEEDED: 3. Install Dioxus CLI
#RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
#RUN cargo binstall dioxus-cli --root /.cargo -y --force
#ENV PATH="/.cargo/bin:$PATH"

# NOT NEEDED: Create the final bundle folder. Bundle with release BUT WE DONT 
# DO THIS ANYMORE BECAUSE THE HOST NOW PRODUCES THE BINARY WHICH HAS THE .git HASH.
#RUN dx bundle --web --release
RUN mkdir /usr/local/app/
WORKDIR /usr/local/app
COPY . .

COPY ./target/dx/Admindash3/release/web/ /usr/local/app
#COPY data_buttons.json /usr/local/app
#COPY data_navbar.json /usr/local/app

# THIS HAS OUR ACTUAL FILES! (TODO: SHOULD RENAME THESE BETTER)
#COPY --from=admindash3-builder /app/target/dx/Admindash3/release/web/ /usr/local/app
#COPY data_buttons.json /usr/local/app
#COPY data_navbar.json /usr/local/app

# set our port and make sure to listen for all connections
ENV PORT=80
ENV IP=0.0.0.0

# expose the port 80
EXPOSE 80

WORKDIR /usr/local/app
ENTRYPOINT [ "/usr/local/app/server" ]
