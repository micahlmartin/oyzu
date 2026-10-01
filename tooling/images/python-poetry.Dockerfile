# CI provisioned toolchain. Oyzu only consumes its resolved immutable image ID.
FROM python:3.12-slim-bookworm
RUN python -m pip install --no-cache-dir poetry==2.5.1 poetry-plugin-export==1.10.1
