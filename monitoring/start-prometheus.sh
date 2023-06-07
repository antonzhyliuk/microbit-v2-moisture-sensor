docker run \
    -d \
    --name=prometheus \
    --restart unless-stopped \
    -p 9090:9090 \
    -v ./prometheus/config.yml:/etc/prometheus/prometheus.yml \
    -v ./prometheus/data:/prometheus \
    prom/prometheus