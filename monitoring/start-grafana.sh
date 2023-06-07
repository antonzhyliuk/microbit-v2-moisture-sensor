docker run \
    -d \
    --name=grafana \
    --restart unless-stopped \
    -p 3000:3000 \
    -v ./grafana/data:/var/lib/grafana \
    grafana/grafana