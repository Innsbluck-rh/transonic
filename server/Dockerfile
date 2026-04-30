FROM gcr.io/distroless/static-debian12:nonroot

COPY transonic-server /usr/bin/transonic-server

EXPOSE 4747
VOLUME ["/data"]

USER nonroot:nonroot
ENTRYPOINT ["/usr/bin/transonic-server"]
CMD ["serve", "--config", "/data/config.json"]
