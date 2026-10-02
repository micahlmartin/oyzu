FROM alpine:3.22
COPY --from=docker.io/library/alpine:3.22 /etc/alpine-release /copied-release
RUN --mount=type=bind,from=docker.io/library/alpine:3.22,source=/etc,target=/base-etc \
    cmp /copied-release /base-etc/alpine-release
