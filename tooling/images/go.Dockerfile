# Explicit toolchain provisioning; project preparation/execution downloads no tools.
FROM golang:1.24-bookworm AS modulezip
WORKDIR /adapter
COPY src/builders/go/runtime/modulezip/ ./
RUN go mod download && go mod verify \
    && CGO_ENABLED=0 go build -mod=readonly -trimpath -buildvcs=false -o /usr/local/bin/oyzu-go-modulezip . \
    && mkdir -p /licenses \
    && cp /go/pkg/mod/golang.org/x/mod@v0.25.0/LICENSE /licenses/golang.org-x-mod-LICENSE

FROM golang:1.24-bookworm
COPY --from=modulezip /usr/local/bin/oyzu-go-modulezip /usr/local/bin/oyzu-go-modulezip
COPY --from=modulezip /licenses/ /usr/local/share/licenses/oyzu-go-modulezip/
