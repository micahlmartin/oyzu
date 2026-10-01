# syntax=docker/dockerfile:1
FROM python:3.13-slim
WORKDIR /app
COPY requirements.txt .
RUN --mount=type=bind,from=dependencies,target=/dependencies \
    pip install --no-index --find-links=/dependencies -r requirements.txt
COPY app.py .
CMD ["python", "app.py"]
