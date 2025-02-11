optimize:
    ./devtools/optimize.sh

network_inspect := "docker network inspect hyperlane-net -f '{{range .Containers}}{{.Name}} {{end}}'"
disconnect-hyperlane-network:
    #!/usr/bin/env sh
    containers=$({{network_inspect}})
    for container in $containers; do
        docker network disconnect hyperlane-net $container
    done
    docker network rm hyperlane-net