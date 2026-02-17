FROM ubuntu:22.04

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV PYTHONUNBUFFERED=1 \
    PYTHONIOENCODING=UTF-8 \
    TZ=America/New_York \
    LANG=en_US.UTF-8 \
    LANGUAGE=en_US:en \
    LC_ALL=en_US.UTF-8 \
    DEBIAN_FRONTEND=noninteractive

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

#====================================================
# Install System Dependencies, Tools, Python & Chrome
#====================================================
RUN apt-get update && \
    # 1. Install software-properties-common
    apt-get install -y --no-install-recommends software-properties-common && \
    # 2. Enable BOTH Universe (for fonts/xvfb) and Multiverse (for rar)
    add-apt-repository -y universe && \
    add-apt-repository -y multiverse && \
    # 3. Update index to see the new packages
    apt-get update && \
    # 4. Install packages
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
    # Fonts (Many of these are in Universe)
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
    # Python (Dependencies often in Universe)
    python3 \
    python3-pip \
    python3-setuptools \
    python3-dev \
    python3-tk \
    && \
    # 5. Generate Locales
    sed -i '/en_US.UTF-8/s/^# //g' /etc/locale.gen && \
    locale-gen en_US.UTF-8 && \
    # 6. Install Chrome
    wget -q https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb && \
    apt-get install -y ./google-chrome-stable_current_amd64.deb && \
    rm google-chrome-stable_current_amd64.deb && \
    # 7. Cleanup
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

RUN ln -s /usr/bin/python3 /usr/bin/python

#================
# Setting up app
#================
WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 10000

# CMD ["python", "main.py"]
