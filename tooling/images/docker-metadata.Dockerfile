FROM golang:1.24-bookworm AS compile
WORKDIR /adapter
COPY src/builders/docker/runtime/metadata/go.mod src/builders/docker/runtime/metadata/go.sum ./
RUN go mod download
COPY src/builders/docker/runtime/metadata/*.go ./
RUN --network=none GOPROXY=off GOSUMDB=off go test -mod=readonly ./... && \
    CGO_ENABLED=0 GOPROXY=off GOSUMDB=off go build -mod=readonly -trimpath -o /oyzu-docker-metadata .
# Preserve dependency notices alongside the linked native SDK adapter.
RUN mkdir /notices && go list -m -f '{{.Path}} {{.Dir}}' all | while read module directory; do \
      if [ -n "$directory" ]; then \
        mkdir -p "/notices/$module"; \
        find "$directory" -maxdepth 1 -type f \( -iname 'license*' -o -iname 'notice*' -o -iname 'copying*' \) -exec cp '{}' "/notices/$module/" \; ; \
      fi; \
    done

FROM moby/buildkit:v0.25.0-rootless
COPY --from=compile /oyzu-docker-metadata /usr/bin/oyzu-docker-metadata
COPY --from=compile /notices /usr/share/oyzu-docker-metadata/notices
