FROM ubuntu:22.04

# Best practice shell setup
SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Environment Variables
ENV PYTHONUNBUFFERED=1 \
    PYTHONIOENCODING=UTF-8 \
    TZ=America/New_York \
    LANG=en_US.UTF-8 \
    LANGUAGE=en_US:en \
    LC_ALL=en_US.UTF-8

# Setup Timezone
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

#====================================================
# Install System Dependencies, Tools, Python & Chrome
#====================================================
RUN apt-get update && \
    # 1. Install software-properties-common to manage repositories
    apt-get install -y --no-install-recommends software-properties-common && \
    # 2. Enable multiverse repo (required for 'rar')
    add-apt-repository multiverse && \
    apt-get update && \
    # 3. Install packages
    apt-get install -y --no-install-recommends \
    locales \
    tzdata \
    ca-certificates \
    sudo \
    wget \
    curl \
    unzip \
    vim \
    rar \
    xvfb \
    # Fonts
    fonts-liberation \
    fonts-liberation2 \
    fonts-font-awesome \
    fonts-ubuntu \
    fonts-terminus \
    fonts-powerline \
    fonts-open-sans \
    fonts-mononoki \
    fonts-roboto \
    fonts-lato \
    # Python Setup (Ubuntu 22.04 has Python 3.10 built-in)
    python3 \
    python3-pip \
    python3-setuptools \
    python3-dev \
    python3-tk \
    && \
    # 4. Generate Locales
    sed -i '/en_US.UTF-8/s/^# //g' /etc/locale.gen && \
    locale-gen en_US.UTF-8 && \
    # 5. Install Chrome (Automatically pulls in needed Linux lib dependencies)
    wget -q https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb && \
    apt-get install -y ./google-chrome-stable_current_amd64.deb && \
    rm google-chrome-stable_current_amd64.deb && \
    # 6. Cleanup
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create symlink for python -> python3
RUN ln -s /usr/bin/python3 /usr/bin/python

#================
# Setting up app
#================
WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

# Exposed on Port 10000
EXPOSE 10000

# CMD ["python", "main.py"]
