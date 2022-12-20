FROM rustembedded/cross:armv7-unknown-linux-gnueabihf-0.2.1

RUN apt-get update
RUN dpkg --add-architecture armhf && \
    apt-get update && \
    apt-get install --assume-yes libssl-dev:armhf libasound2-dev:armhf pkg-config:armhf alsa:armhf
RUN apt-get install --assume-yes libasound-dev:armhf

ENV PKG_CONFIG_LIBDIR_armv7_unknown_linux_gnueabihf=/usr/lib/arm-linux-gnueabihf/pkgconfig