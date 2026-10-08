# This dockerfile requires that your compilation environment already have 
# git, dioxus, and the sourcecode installed and ready. It also requires that 
# you have built the latest dioxus and wasm builds via running:
# dx bundle --web --release 
#
# This dockerfile no longer uses the Dioxus docs official procedures b/c the git hash is embedded
# in the container, was: https://dioxuslabs.com/learn/0.7/tutorial/deploy



FROM ubuntu
#FROM alpine  # IMPORTANT: <--- I cant use this because Alpine does not normally have the glibc loader /lib64/ld-linux-x86-x64...

# DISABLED THIS SECTION TO SHRINK THE UBUNTU CONTAINER TO 40M
# 1. Install system dependencies, git, and libraries needed to build Rust crates 
#RUN apt-get update && apt-get install -y \
#    curl \
#    git \
#    vim \
#    less \
#    && rm -rf /var/lib/apt/lists/*


# Use this host compiled binary and source directories (x86-64) directly in the container (instead of using the rust:latest container).
# This is not following the dioxus guide, because we need the git revparse environment variable at compile time, unsure of how to 
# get around it at this time.
RUN mkdir /usr/local/app/
WORKDIR /usr/local/app
COPY . .

# Ensure that the ./server binary is at the top level, this one is actually used (we should really do a MOVE not a COPY here).
COPY ./target/dx/Admindash3/release/web/ /usr/local/app

# Set our port and make sure to listen for all connections
ENV PORT=80
ENV IP=0.0.0.0

# Expose the port 80
EXPOSE 80

WORKDIR /usr/local/app
ENTRYPOINT [ "/usr/local/app/server" ]
