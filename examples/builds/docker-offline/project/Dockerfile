FROM scratch AS contents
COPY greeting.txt /greeting.txt
FROM scratch
COPY --from=contents /greeting.txt /greeting.txt
