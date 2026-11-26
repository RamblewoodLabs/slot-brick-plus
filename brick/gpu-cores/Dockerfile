FROM ubuntu:20.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends gcc-10 g++-10 make ninja-build python3 python3-pip git patchelf pkg-config ca-certificates libgles2-mesa-dev libegl1-mesa-dev binutils file && \
    pip3 install cmake==3.28.4 && \
    update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-10 100 && update-alternatives --install /usr/bin/g++ g++ /usr/bin/g++-10 100 && \
    update-alternatives --install /usr/bin/cc cc /usr/bin/gcc-10 100 && update-alternatives --install /usr/bin/c++ c++ /usr/bin/g++-10 100 && rm -rf /var/lib/apt/lists/*
