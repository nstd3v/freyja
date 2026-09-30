FROM debian:bookworm
RUN apt-get update && apt-get install -y --no-install-recommends nginx && rm -rf /var/lib/apt/lists/*
CMD ["nginx", "-g", "daemon off;"]
