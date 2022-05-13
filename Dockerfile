FROM rust:1.60 AS build
RUN mkdir /src
COPY . /src
WORKDIR /src
RUN cargo build --release 

FROM debian:stable-slim
ENV TINI_URL="https://github.com/krallin/tini/releases/download/v0.18.0/tini-static-amd64"
ADD ${TINI_URL} /tini
RUN chmod a+x /tini

COPY --from=build /src/target/release/codeowner-gen /usr/bin/codeowner-gen
ENTRYPOINT ["/tini", "--"]
CMD ["/usr/bin/codeowner-gen"]
