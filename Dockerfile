FROM ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf

RUN dpkg --add-architecture armhf
RUN apt-get update --assume-yes 
RUN apt-get install --assume-yes libasound2-dev:armhf