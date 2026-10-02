FROM alpine:3.22
COPY greeting.txt /greeting.txt
RUN test -f /etc/alpine-release && test ! -e /inputs && test ! -e /var/run/docker.sock
