from rust:1.41 as build

WORKDIR /src
COPY . .
RUN rustup component add rustfmt
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=build /usr/local/cargo/bin/kubeware /usr/local/bin/kubeware
COPY --from=build /src/config.toml /opt/kubeware/config.toml
ENV CONFIG_FILE /opt/kubeware/config.toml

CMD ["kubeware"]