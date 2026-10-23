FROM python:3.11-slim

WORKDIR /app

# Install system libraries needed by librasgapdf (libfontconfig1, etc.)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libfontconfig1 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Create non-root user required by Hugging Face Spaces (UID 1000)
RUN useradd -m -u 1000 user
USER user
ENV HOME=/home/user \
    PATH=/home/user/.local/bin:$PATH \
    PYTHONUNBUFFERED=1

WORKDIR /app

COPY --chown=user:user . /app

EXPOSE 7860

CMD ["python3", "server.py"]
