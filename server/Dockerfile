FROM python:3.10-slim-bookworm

# Best practice shell setup
SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV PYTHONUNBUFFERED=1 \
    PYTHONIOENCODING=UTF-8 \
    # Set display for Xvfb
    DISPLAY=:99

#====================================================
# Install System Dependencies & Chromium (ARM64 safe)
#====================================================
RUN apt-get update && \
    # 1. Install utils & enable non-free (for rar if available)
    apt-get install -y --no-install-recommends \
    software-properties-common \
    gnupg \
    wget \
    curl \
    unzip \
    vim \
    xvfb \
    # 2. Install Chromium & Driver (Native ARM64 support on Debian)
    chromium \
    chromium-driver \
    # 3. Fonts
    fonts-liberation \
    fonts-font-awesome \
    fonts-dejavu \
    fonts-open-sans \
    && \
    # 4. Try to install RAR (ARM64 support varies, unrar is safer fallback)
    apt-get install -y --no-install-recommends unrar || true && \
    # Cleanup
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

#================
# Setting up app
#================
WORKDIR /app

# Upgrade pip
RUN pip install --no-cache-dir --upgrade pip

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 10000

# Start Xvfb in the background and then your app
# (You might need a small entrypoint script for this, or handle it in python)
# CMD Xvfb :99 -screen 0 1280x1024x16 & python main.pyEXPOSE 10000

# CMD ["python", "main.py"]
