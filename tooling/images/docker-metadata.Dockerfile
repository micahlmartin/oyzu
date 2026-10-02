FROM python:3.12-slim-bookworm AS lint
COPY tooling/provision-hadolint.py /provision-hadolint.py
COPY tooling/provisioning.py /provisioning.py
RUN python /provision-hadolint.py --destination /tools

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

WORKDIR /images
COPY src/dependencies/runtime/images/go.mod src/dependencies/runtime/images/go.sum ./
RUN go mod download && go mod verify
COPY src/dependencies/runtime/images/*.go ./
RUN --network=none GOPROXY=off GOSUMDB=off go test -mod=readonly ./... && \
    CGO_ENABLED=0 GOPROXY=off GOSUMDB=off go build -mod=readonly -trimpath -o /oyzu-docker-images .
RUN mkdir /image-notices && go list -m -f '{{.Path}} {{.Dir}}' all | while read module directory; do \
      if [ -n "$directory" ]; then \
        mkdir -p "/image-notices/$module"; \
        find "$directory" -maxdepth 1 -type f \( -iname 'license*' -o -iname 'notice*' -o -iname 'copying*' \) -exec cp '{}' "/image-notices/$module/" \; ; \
      fi; \
    done

WORKDIR /quality
COPY tooling/images/docker-quality/go.mod tooling/images/docker-quality/go.sum ./
RUN go mod download && go mod verify
RUN --network=none CGO_ENABLED=0 GOPROXY=off GOSUMDB=off go build -mod=readonly -trimpath -o /dockerfmt github.com/reteps/dockerfmt
RUN mkdir /quality-notices && go list -m -f '{{.Path}} {{.Dir}}' all | while read module directory; do \
      if [ -n "$directory" ]; then \
        mkdir -p "/quality-notices/$module"; \
        find "$directory" -maxdepth 1 -type f \( -iname 'license*' -o -iname 'notice*' -o -iname 'copying*' \) -exec cp '{}' "/quality-notices/$module/" \; ; \
      fi; \
    done

FROM moby/buildkit:v0.25.0-rootless
COPY --from=compile /oyzu-docker-metadata /usr/bin/oyzu-docker-metadata
COPY --from=compile /oyzu-docker-images /usr/bin/oyzu-docker-images
COPY --from=compile /image-notices /usr/share/oyzu-docker-images/notices
COPY --from=compile /notices /usr/share/oyzu-docker-metadata/notices
COPY --from=compile /dockerfmt /usr/bin/dockerfmt
COPY --from=compile /quality-notices /usr/share/oyzu-docker-quality/notices
COPY --from=lint /tools/hadolint /usr/bin/hadolint
COPY --from=lint /tools/hadolint-LICENSE /tools/hadolint-source.txt /usr/share/oyzu-docker-quality/
RUN hadolint --version && dockerfmt version
