FROM rust:1.87

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    sudo \
    pkg-config \
    libssl-dev \
    ca-certificates \
    lldb \
    git && \
    rm -rf /var/lib/apt/lists/*


RUN useradd -m vscode && \
    echo "vscode ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers