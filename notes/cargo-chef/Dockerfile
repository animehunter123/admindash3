# This dockerfile uses the Dioxus docs official procedures to 
# build our container, see: https://dioxuslabs.com/learn/0.7/tutorial/deploy
# 
# I OPTED TO NO LONGER USE THIS FOR EMBEDDING OUR GIT HASH INTO THE EXE RUNTIME 
# SINCE IT REQUIRED THE CARGO COMMAND TO BE ABLE TO RUN GIT FETCH/HASH AT 
# RUNTIME.

# PHASE1 Download and cache the rust dependencies for incremental builds!
FROM rust:1 AS admindash3-chef
RUN cargo install cargo-chef
WORKDIR /app

FROM admindash3-chef AS admindash3-planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# PHASE2 Use cargo chef to load the cached dependencies and BUILD OUR CODE NOW!!
FROM admindash3-chef AS admindash3-builder
COPY --from=admindash3-planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .

# Install `dx`
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"

# Create the final bundle folder. Bundle with release build profile to enable optimizations.
RUN dx bundle --web --release

# PHASE3 copy the release web folder to slim runtime to serve our app!!!!!!!!!!!
FROM admindash3-chef AS admindash3-runtime

# THIS HAS OUR ACTUAL FILES! (TODO: SHOULD RENAME THESE BETTER)
COPY --from=admindash3-builder /app/target/dx/admindash3/release/web/ /usr/local/app
COPY data_buttons.json /usr/local/app
COPY data_navbar.json /usr/local/app

# set our port and make sure to listen for all connections
ENV PORT=80
ENV IP=0.0.0.0

# expose the port 80
EXPOSE 80

WORKDIR /usr/local/app
ENTRYPOINT [ "/usr/local/app/server" ]
